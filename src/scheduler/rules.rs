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

use chrono::{Local, NaiveDate, NaiveTime, Timelike};
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
    /// 用户手动结束会话后，当天不再被自动排班重新启动。
    manual_stop_date: Option<NaiveDate>,
    /// 当前会话归属日期；跨天工作窗口会先收尾旧会话再启动新会话。
    session_date: Option<NaiveDate>,
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
            manual_stop_date: None,
            session_date: None,
        }
    }

    pub fn apply(&mut self, cmd: Command, cfg: &Config) -> ApplyOutcome {
        let mut out = ApplyOutcome::default();
        match cmd {
            Command::Start => {
                if self.state == RunState::Idle {
                    self.reset_session();
                }
                let now = Local::now();
                self.manual_stop_date = None;
                self.session_date = schedule_day(
                    &cfg.general.work_start,
                    &cfg.general.work_end,
                    now.date_naive(),
                    now.time(),
                )
                .or(Some(now.date_naive()));
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
                out.session_ended_secs = Some(self.running_secs);
                let now = Local::now();
                self.manual_stop_date = schedule_day(
                    &cfg.general.work_start,
                    &cfg.general.work_end,
                    now.date_naive(),
                    now.time(),
                )
                .or(Some(now.date_naive()));
                self.state = RunState::Idle;
                self.session_date = None;
                self.clear_runtime_after_stop();
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

    fn reset_session(&mut self) {
        self.running_secs = 0;
        self.last_fire.clear();
        self.phase = Phase::Focus;
        self.big_break_secs = 0;
        self.last_heartbeat = 0;
        self.fired_today.clear();
        self.off_work_fired_date = None;
        self.last_emit_secs = None;
        self.pending_micro.clear();
    }

    fn clear_runtime_after_stop(&mut self) {
        self.running_secs = 0;
        self.big_break_secs = 0;
        self.last_emit_secs = None;
        self.pending_micro.clear();
    }

    pub fn tick(&mut self, _now: Instant, cfg: &Config) -> TickOutcome {
        let local_now = Local::now();
        self.tick_at(cfg, local_now.date_naive(), local_now.time())
    }

    fn tick_at(&mut self, cfg: &Config, today_date: NaiveDate, now_time: NaiveTime) -> TickOutcome {
        let mut out = TickOutcome::default();

        if cfg.general.auto_schedule {
            let work_day = schedule_day(
                &cfg.general.work_start,
                &cfg.general.work_end,
                today_date,
                now_time,
            );
            let Some(work_day) = work_day else {
                if self.state != RunState::Idle {
                    out.session_ended_secs = Some(self.running_secs);
                    self.state = RunState::Idle;
                    self.session_date = None;
                    self.clear_runtime_after_stop();
                    out.state_changed = Some(RunState::Idle);
                }
                return out;
            };
            if self.state != RunState::Idle && self.session_date != Some(work_day) {
                out.session_ended_secs = Some(self.running_secs);
                self.state = RunState::Idle;
                self.session_date = None;
                self.clear_runtime_after_stop();
                out.state_changed = Some(RunState::Idle);
                return out;
            }
            // 每天首次进入工作窗口自动开始；当天手动停止后保持空闲。
            if self.state == RunState::Idle
                && self.manual_stop_date != Some(work_day)
            {
                self.reset_session();
                self.state = RunState::Running;
                self.session_date = Some(work_day);
                out.state_changed = Some(RunState::Running);
            }
        }

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

        let in_quiet = in_quiet_hours(
            &cfg.general.quiet_start,
            &cfg.general.quiet_end,
            now_time,
        );

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
        let today = today_date.format("%Y-%m-%d").to_string();
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

/// 返回当前工作窗口所属的日期；跨午夜时，00:00–结束时间归到前一天窗口。
fn schedule_day(start: &str, end: &str, date: NaiveDate, now: NaiveTime) -> Option<NaiveDate> {
    let (Some(s), Some(e)) = (parse_hhmm(start), parse_hhmm(end)) else {
        return None;
    };
    if s == e {
        return None;
    }
    if s < e {
        (now >= s && now < e).then_some(date)
    } else if now >= s {
        Some(date)
    } else if now < e {
        date.pred_opt()
    } else {
        None
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
        if fired.insert(key, ()).is_none() {
            triggered.push(kind);
        }
    }
}

/// 判断当前本地时间是否处于勿扰时段（支持跨夜的形式）
fn in_quiet_hours(start: &str, end: &str, now: NaiveTime) -> bool {
    let (Some(s), Some(e)) = (parse_hhmm(start), parse_hhmm(end)) else {
        return false;
    };
    if s == e {
        return false;
    }
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
        // 规则单测按手动状态机推进，自动排班由独立用例覆盖。
        cfg.general.auto_schedule = false;
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

    #[test]
    fn heartbeat_restarts_from_zero_after_session_restart() {
        let mut e = Engine::new();
        let cfg = fast_cfg();

        e.apply(Command::Start, &cfg);
        for _ in 0..4 {
            assert!(e.tick(Instant::now(), &cfg).heartbeat.is_none());
        }
        assert_eq!(e.tick(Instant::now(), &cfg).heartbeat, Some(5));

        e.apply(Command::Stop, &cfg);
        e.apply(Command::Start, &cfg);
        for _ in 0..4 {
            assert!(e.tick(Instant::now(), &cfg).heartbeat.is_none());
        }
        assert_eq!(e.tick(Instant::now(), &cfg).heartbeat, Some(5));
    }

    #[test]
    fn work_window_includes_start_and_excludes_end() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();
        assert_eq!(
            schedule_day("09:00", "19:00", date, NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
            Some(date)
        );
        assert_eq!(
            schedule_day(
                "09:00",
                "19:00",
                date,
                NaiveTime::from_hms_opt(18, 59, 59).unwrap()
            ),
            Some(date)
        );
        assert_eq!(
            schedule_day("09:00", "19:00", date, NaiveTime::from_hms_opt(19, 0, 0).unwrap()),
            None
        );
        assert_eq!(
            schedule_day("09:00", "19:00", date, NaiveTime::from_hms_opt(8, 59, 59).unwrap()),
            None
        );
    }

    #[test]
    fn work_window_supports_overnight_and_rejects_equal_times() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();
        assert_eq!(
            schedule_day("22:00", "06:00", date, NaiveTime::from_hms_opt(23, 0, 0).unwrap()),
            Some(date)
        );
        assert_eq!(
            schedule_day(
                "22:00",
                "06:00",
                date,
                NaiveTime::from_hms_opt(5, 59, 59).unwrap()
            ),
            date.pred_opt()
        );
        assert_eq!(
            schedule_day("09:00", "09:00", date, NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
            None
        );
    }

    #[test]
    fn auto_schedule_starts_stops_and_respects_manual_stop() {
        let mut e = Engine::new();
        let mut cfg = fast_cfg();
        cfg.general.auto_schedule = true;
        cfg.general.work_start = "09:00".into();
        cfg.general.work_end = "19:00".into();
        let today = Local::now().date_naive();

        let before = e.tick_at(&cfg, today, NaiveTime::from_hms_opt(8, 59, 59).unwrap());
        assert!(before.state_changed.is_none());

        let started = e.tick_at(&cfg, today, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        assert_eq!(started.state_changed, Some(RunState::Running));

        e.apply(Command::Stop, &cfg);
        let still_idle = e.tick_at(&cfg, today, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        assert!(still_idle.state_changed.is_none());
        assert_eq!(e.state, RunState::Idle);

        let tomorrow = today.succ_opt().unwrap();
        let next_day = e.tick_at(&cfg, tomorrow, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        assert_eq!(next_day.state_changed, Some(RunState::Running));

        let stopped = e.tick_at(&cfg, tomorrow, NaiveTime::from_hms_opt(19, 0, 0).unwrap());
        assert_eq!(stopped.state_changed, Some(RunState::Idle));
        assert!(stopped.session_ended_secs.is_some());
    }

    #[test]
    fn manual_pause_is_not_overridden_by_auto_schedule() {
        let mut e = Engine::new();
        let mut cfg = fast_cfg();
        cfg.general.auto_schedule = true;
        let today = Local::now().date_naive();

        e.tick_at(&cfg, today, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        e.apply(Command::Pause, &cfg);
        let out = e.tick_at(&cfg, today, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        assert!(out.state_changed.is_none());
        assert_eq!(e.state, RunState::Paused);
    }

    #[test]
    fn toggling_auto_schedule_off_then_on_can_start_again() {
        let mut e = Engine::new();
        let mut cfg = fast_cfg();
        cfg.general.auto_schedule = true;
        let today = Local::now().date_naive();

        e.tick_at(&cfg, today, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        e.tick_at(&cfg, today, NaiveTime::from_hms_opt(19, 0, 0).unwrap());
        cfg.general.auto_schedule = false;
        e.tick_at(&cfg, today, NaiveTime::from_hms_opt(19, 1, 0).unwrap());
        cfg.general.auto_schedule = true;
        cfg.general.work_end = "20:00".into();
        let restarted = e.tick_at(&cfg, today, NaiveTime::from_hms_opt(19, 1, 0).unwrap());
        assert_eq!(restarted.state_changed, Some(RunState::Running));
    }

    #[test]
    fn overnight_schedule_keeps_session_across_midnight() {
        let mut e = Engine::new();
        let mut cfg = fast_cfg();
        cfg.general.auto_schedule = true;
        cfg.general.work_start = "22:00".into();
        cfg.general.work_end = "06:00".into();
        let date = NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();

        let started = e.tick_at(&cfg, date, NaiveTime::from_hms_opt(22, 0, 0).unwrap());
        assert_eq!(started.state_changed, Some(RunState::Running));
        for _ in 0..3 {
            e.tick_at(&cfg, date.succ_opt().unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        }
        assert_eq!(e.state, RunState::Running);

        let stopped = e.tick_at(
            &cfg,
            date.succ_opt().unwrap(),
            NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        );
        assert_eq!(stopped.state_changed, Some(RunState::Idle));
    }
}
