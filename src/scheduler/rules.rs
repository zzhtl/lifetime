// 规则匹配引擎
//
// 设计思想：
//   - 每种周期型提醒维护 last_fired_at (Instant)；
//   - tick 时检查 elapsed >= interval 即触发；
//   - 番茄钟用单独的 phase 状态（Focus/Break）来回切；
//   - 大休息独立计时，连续工作满 N 分钟即触发并重置；
//   - 时间点型（午餐/睡眠）按本地时钟 HH:MM 匹配，每天只触发一次。
//   - 微提醒（护眼/起身/喝水/颈椎）做全局错开：到点先入队，按 min_notify_gap_sec
//     每隔一段只补发一条（FIFO，公平轮转），保证该间隔内不出现两次微提醒。

use chrono::{Local, NaiveTime, Timelike};
use std::collections::HashMap;
use std::time::Instant;

use crate::config::{parse_hhmm, Config};
use crate::reminders::ReminderKind;
use crate::scheduler::event::{ApplyOutcome, Command, RunState, TickOutcome};

/// 番茄钟阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Focus,
    Break,
}

pub struct Engine {
    state: RunState,
    /// 当前会话累计秒数（仅在 Running 时增长）
    running_secs: u64,
    /// 各种周期型提醒上次触发的相对时刻（按 running_secs 计）
    last_fire: HashMap<ReminderKind, u64>,
    /// 番茄钟当前阶段（仅 Pomodoro 启用时使用）
    phase: Phase,
    /// 大休息独立计时（连续工作秒数，被大休息或暂停重置）
    big_break_secs: u64,
    /// 上次心跳上报时间（避免每秒刷屏）
    last_heartbeat: u64,
    /// 时间点型提醒今日是否已触发，按 (kind, YMD) 记
    fired_today: HashMap<(ReminderKind, String), ()>,
    /// 是否已发出过 OffWork 提醒（每天一次）
    off_work_fired_date: Option<String>,
    /// 上次"发出任意通知"的相对时刻（按 running_secs 计），用于微提醒错开
    last_emit_secs: Option<u64>,
    /// 已到点但被错开推迟的微提醒队列（FIFO，按种类去重，至多 4 条）
    pending_micro: Vec<ReminderKind>,
}

/// 受全局错开约束的微提醒种类（到点先入队，按最小间隔逐条补发）
const MICRO_KINDS: [ReminderKind; 4] = [
    ReminderKind::Eyes,
    ReminderKind::Stand,
    ReminderKind::Water,
    ReminderKind::Neck,
];

impl Engine {
    pub fn new() -> Self {
        Self {
            state: RunState::Idle,
            running_secs: 0,
            last_fire: HashMap::new(),
            phase: Phase::Focus,
            big_break_secs: 0,
            last_heartbeat: 0,
            fired_today: HashMap::new(),
            off_work_fired_date: None,
            last_emit_secs: None,
            pending_micro: Vec::new(),
        }
    }

    pub fn apply(&mut self, cmd: Command, cfg: &Config) -> ApplyOutcome {
        let mut out = ApplyOutcome::default();
        match cmd {
            Command::Start => {
                if self.state == RunState::Idle {
                    self.running_secs = 0;
                    self.last_fire.clear();
                    self.phase = Phase::Focus;
                    self.big_break_secs = 0;
                    self.fired_today.clear();
                    self.off_work_fired_date = None;
                    self.last_emit_secs = None;
                    self.pending_micro.clear();
                }
                self.state = RunState::Running;
                out.state_changed = Some(self.state);
            }
            Command::Pause => {
                if self.state == RunState::Running {
                    self.state = RunState::Paused;
                    out.state_changed = Some(self.state);
                }
            }
            Command::Resume => {
                if self.state == RunState::Paused {
                    self.state = RunState::Running;
                    out.state_changed = Some(self.state);
                }
            }
            Command::Stop => {
                self.state = RunState::Idle;
                self.running_secs = 0;
                self.big_break_secs = 0;
                self.last_emit_secs = None;
                self.pending_micro.clear();
                out.state_changed = Some(self.state);
            }
            Command::Skip(kind) => {
                self.last_fire.insert(kind, self.running_secs);
                self.pending_micro.retain(|k| *k != kind);
                if kind == ReminderKind::BigBreak {
                    self.big_break_secs = 0;
                }
            }
            Command::Snooze(kind, dur) => {
                // 推迟：把"上次触发"往后挪一个 snooze 距离，本质等同于推迟触发
                let interval = cfg.reminders.interval_sec(kind).unwrap_or(0);
                let push = interval.saturating_sub(dur.as_secs());
                self.last_fire.insert(kind, self.running_secs.saturating_sub(push));
            }
            Command::AcknowledgeBreak(kind) => {
                self.last_fire.insert(kind, self.running_secs);
                self.pending_micro.retain(|k| *k != kind);
                if kind == ReminderKind::BigBreak {
                    self.big_break_secs = 0;
                }
                if matches!(kind, ReminderKind::PomodoroBreak | ReminderKind::PomodoroFocus) {
                    self.phase = match self.phase {
                        Phase::Focus => Phase::Break,
                        Phase::Break => Phase::Focus,
                    };
                }
            }
            Command::TriggerNow(kind) => {
                out.triggered = Some(kind);
            }
            // 出声/测试类指令不影响调度状态，由 run_loop 直接处理副作用
            Command::TestSound | Command::TestNotify | Command::Beep(_) | Command::Quit => {}
        }
        out
    }

    pub fn tick(&mut self, _now: Instant, cfg: &Config) -> TickOutcome {
        let mut out = TickOutcome::default();
        if self.state != RunState::Running {
            return out;
        }

        self.running_secs += 1;
        self.big_break_secs += 1;

        // 心跳每 5 秒上报，减小 channel 流量
        if self.running_secs.saturating_sub(self.last_heartbeat) >= 5 {
            self.last_heartbeat = self.running_secs;
            out.heartbeat = Some(self.running_secs);
        }

        let in_quiet = in_quiet_hours(&cfg.general.quiet_start, &cfg.general.quiet_end);

        // 1) 周期型微提醒：到点不直接触发，而是入队（FIFO，去重），由末尾错开逻辑补发
        for kind in MICRO_KINDS {
            if !cfg.reminders.is_enabled(kind) {
                continue;
            }
            if in_quiet {
                continue;
            }
            let interval = cfg.reminders.interval_sec(kind).unwrap_or(u64::MAX);
            let last = self.last_fire.get(&kind).copied().unwrap_or(0);
            if self.running_secs.saturating_sub(last) >= interval {
                // 周期照常推进；是否已在队列里则避免重复入队
                self.last_fire.insert(kind, self.running_secs);
                if !self.pending_micro.contains(&kind) {
                    self.pending_micro.push(kind);
                }
            }
        }

        // 2) 番茄钟：当前阶段满了 → 切阶段并触发对应事件
        if cfg.reminders.enabled.pomodoro {
            let (cur_kind, target_phase, interval) = match self.phase {
                Phase::Focus => (
                    ReminderKind::PomodoroBreak,
                    Phase::Break,
                    cfg.reminders.pomodoro_focus_sec,
                ),
                Phase::Break => (
                    ReminderKind::PomodoroFocus,
                    Phase::Focus,
                    cfg.reminders.pomodoro_break_sec,
                ),
            };
            let last = self.last_fire.get(&cur_kind).copied().unwrap_or(0);
            if self.running_secs.saturating_sub(last) >= interval {
                self.last_fire.insert(cur_kind, self.running_secs);
                self.phase = target_phase;
                out.triggered.push(cur_kind);
            }
        }

        // 3) 大休息：连续工作满则触发（即便勿扰时段也照触发，因为强制）
        if cfg.reminders.enabled.big_break
            && self.big_break_secs >= cfg.reminders.big_break_interval_sec
        {
            self.big_break_secs = 0;
            self.last_fire.insert(ReminderKind::BigBreak, self.running_secs);
            out.triggered.push(ReminderKind::BigBreak);
        }

        // 4) 时间点型：午餐 / 睡眠（按本地时钟）
        let today = Local::now().format("%Y-%m-%d").to_string();
        let now_time = Local::now().time();
        check_time_point(
            ReminderKind::Lunch,
            &cfg.reminders.lunch_time,
            cfg.reminders.enabled.lunch,
            &today,
            now_time,
            &mut self.fired_today,
            &mut out.triggered,
        );
        check_time_point(
            ReminderKind::Sleep,
            &cfg.reminders.sleep_time,
            cfg.reminders.enabled.sleep,
            &today,
            now_time,
            &mut self.fired_today,
            &mut out.triggered,
        );

        // 5) 累计型：工作满 8h 提醒下班
        if cfg.reminders.enabled.off_work
            && self.running_secs >= cfg.reminders.off_work_total_sec
            && self.off_work_fired_date.as_deref() != Some(&today)
        {
            self.off_work_fired_date = Some(today.clone());
            out.triggered.push(ReminderKind::OffWork);
        }

        // 6) 微提醒错开补发：
        //    本 tick 若已发出结构型/定点型提醒（番茄钟/大休息/午餐/睡眠/下班），刷新错开
        //    计时并让微提醒让位，避免紧贴其后；否则在满足全局最小间隔时补发队首一条。
        if !out.triggered.is_empty() {
            self.last_emit_secs = Some(self.running_secs);
        } else {
            let min_gap = cfg.general.min_notify_gap_sec;
            let gap_ok = min_gap == 0
                || self
                    .last_emit_secs
                    .is_none_or(|t| self.running_secs.saturating_sub(t) >= min_gap);
            if gap_ok && !self.pending_micro.is_empty() {
                let kind = self.pending_micro.remove(0);
                out.triggered.push(kind);
                self.last_emit_secs = Some(self.running_secs);
            }
        }

        out
    }

    #[allow(dead_code)]
    pub fn running_secs(&self) -> u64 {
        self.running_secs
    }
}

fn check_time_point(
    kind: ReminderKind,
    hhmm: &str,
    enabled: bool,
    today: &str,
    now: NaiveTime,
    fired: &mut HashMap<(ReminderKind, String), ()>,
    triggered: &mut Vec<ReminderKind>,
) {
    if !enabled {
        return;
    }
    let Some(target) = parse_hhmm(hhmm) else {
        return;
    };
    // 在目标时间所在分钟内（second 不限）且当日未触发过
    if now.hour() == target.hour() && now.minute() == target.minute() {
        let key = (kind, today.to_string());
        if !fired.contains_key(&key) {
            fired.insert(key, ());
            triggered.push(kind);
        }
    }
}

/// 判断当前本地时间是否处于勿扰时段（支持跨夜的形式）
fn in_quiet_hours(start: &str, end: &str) -> bool {
    let (Some(s), Some(e)) = (parse_hhmm(start), parse_hhmm(end)) else {
        return false;
    };
    if s == e {
        return false;
    }
    let now = Local::now().time();
    if s < e {
        now >= s && now < e
    } else {
        // 跨夜：22:00 - 06:00
        now >= s || now < e
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn fast_cfg() -> Config {
        let mut cfg = Config::default();
        // 加速测试：把所有周期改为秒级
        cfg.reminders.eyes_interval_sec = 3;
        cfg.reminders.stand_interval_sec = 5;
        cfg.reminders.water_interval_sec = 7;
        cfg.reminders.neck_interval_sec = 11;
        cfg.reminders.pomodoro_focus_sec = 13;
        cfg.reminders.pomodoro_break_sec = 4;
        cfg.reminders.big_break_interval_sec = 20;
        cfg.reminders.big_break_duration_sec = 2;
        // 勿扰时段置空以免影响
        cfg.general.quiet_start = "00:00".into();
        cfg.general.quiet_end = "00:00".into();
        // 关闭错开，保证既有用例按"到点即触发"的语义验证
        cfg.general.min_notify_gap_sec = 0;
        cfg
    }

    #[test]
    fn idle_state_no_tick() {
        let mut e = Engine::new();
        let cfg = fast_cfg();
        for _ in 0..100 {
            let out = e.tick(Instant::now(), &cfg);
            assert!(out.triggered.is_empty());
            assert!(out.heartbeat.is_none());
        }
    }

    #[test]
    fn start_then_eyes_fires_after_interval() {
        let mut e = Engine::new();
        let cfg = fast_cfg();
        let out = e.apply(Command::Start, &cfg);
        assert_eq!(out.state_changed, Some(RunState::Running));
        // 期望至少出现一次 Eyes
        let mut got_eyes = false;
        for _ in 0..3 {
            if e.tick(Instant::now(), &cfg)
                .triggered
                .contains(&ReminderKind::Eyes)
            {
                got_eyes = true;
            }
        }
        assert!(got_eyes, "未触发 Eyes 提醒");
    }

    #[test]
    fn big_break_independent_of_other_intervals() {
        let mut e = Engine::new();
        let cfg = fast_cfg();
        e.apply(Command::Start, &cfg);
        let mut got_big = false;
        for _ in 0..cfg.reminders.big_break_interval_sec {
            if e.tick(Instant::now(), &cfg)
                .triggered
                .contains(&ReminderKind::BigBreak)
            {
                got_big = true;
            }
        }
        assert!(got_big);
    }

    #[test]
    fn micro_reminders_are_staggered() {
        // 护眼与起身在同一时刻到点，但启用了最小间隔 → 同 tick 只发一个，
        // 另一个要等满间隔后才补发，且不丢失。
        let mut e = Engine::new();
        let mut cfg = fast_cfg();
        cfg.reminders.eyes_interval_sec = 2;
        cfg.reminders.stand_interval_sec = 2;
        cfg.reminders.water_interval_sec = 9999;
        cfg.reminders.neck_interval_sec = 9999;
        cfg.reminders.enabled.pomodoro = false;
        cfg.reminders.enabled.big_break = false;
        cfg.reminders.enabled.lunch = false;
        cfg.reminders.enabled.sleep = false;
        cfg.reminders.enabled.off_work = false;
        cfg.general.min_notify_gap_sec = 4;
        e.apply(Command::Start, &cfg);

        // t=1,2：t=2 时两者同刻到点，应只补发一条
        let mut emitted = Vec::new();
        for _ in 0..2 {
            emitted.extend(e.tick(Instant::now(), &cfg).triggered);
        }
        assert_eq!(emitted.len(), 1, "同刻两个微提醒应只发一个");

        // t=3,4,5：仍在间隔内，不应补发第二条
        for _ in 0..3 {
            assert!(
                e.tick(Instant::now(), &cfg).triggered.is_empty(),
                "最小间隔内不应补发第二个微提醒"
            );
        }

        // t=6：满 4 秒间隔，补发第二条
        let out = e.tick(Instant::now(), &cfg);
        assert_eq!(out.triggered.len(), 1, "满间隔后应补发队列中的第二个");
    }

    #[test]
    fn pause_freezes_running_secs() {
        let mut e = Engine::new();
        let cfg = fast_cfg();
        e.apply(Command::Start, &cfg);
        for _ in 0..5 {
            e.tick(Instant::now(), &cfg);
        }
        e.apply(Command::Pause, &cfg);
        let snapshot = e.running_secs();
        for _ in 0..10 {
            e.tick(Instant::now(), &cfg);
        }
        assert_eq!(e.running_secs(), snapshot);
    }
}
