// 规则匹配引擎
//
// 设计思想：
//   - 每种周期型提醒维护 last_fired_at (Instant)；
//   - tick 时检查 elapsed >= interval 即触发；
//   - 番茄钟用单独的 phase 状态（Focus/Break）来回切；
//   - 大休息独立计时，连续工作满 N 分钟即触发并重置；
//   - 时间点型（午餐/睡眠）按本地时钟 HH:MM 匹配，每天只触发一次。

use chrono::{Local, NaiveTime, Timelike};
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::time::Instant;

use crate::config::{parse_hhmm, Config};
use crate::reminders::ReminderKind;
use crate::scheduler::event::{Command, RunState, SchedulerEvent};

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
}

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
        }
    }

    pub fn apply(&mut self, cmd: Command, cfg: &Config, tx: &Sender<SchedulerEvent>) {
        match cmd {
            Command::Start => {
                if self.state == RunState::Idle {
                    self.running_secs = 0;
                    self.last_fire.clear();
                    self.phase = Phase::Focus;
                    self.big_break_secs = 0;
                    self.fired_today.clear();
                    self.off_work_fired_date = None;
                }
                self.state = RunState::Running;
                let _ = tx.send(SchedulerEvent::StateChanged(self.state));
            }
            Command::Pause => {
                if self.state == RunState::Running {
                    self.state = RunState::Paused;
                    let _ = tx.send(SchedulerEvent::StateChanged(self.state));
                }
            }
            Command::Resume => {
                if self.state == RunState::Paused {
                    self.state = RunState::Running;
                    let _ = tx.send(SchedulerEvent::StateChanged(self.state));
                }
            }
            Command::Stop => {
                self.state = RunState::Idle;
                self.running_secs = 0;
                self.big_break_secs = 0;
                let _ = tx.send(SchedulerEvent::StateChanged(self.state));
            }
            Command::Skip(kind) => {
                self.last_fire.insert(kind, self.running_secs);
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
                let _ = tx.send(SchedulerEvent::Triggered(kind));
            }
            Command::Quit => {}
        }
    }

    pub fn tick(&mut self, _now: Instant, cfg: &Config, tx: &Sender<SchedulerEvent>) {
        if self.state != RunState::Running {
            return;
        }

        self.running_secs += 1;
        self.big_break_secs += 1;

        // 心跳每 5 秒上报，减小 channel 流量
        if self.running_secs.saturating_sub(self.last_heartbeat) >= 5 {
            self.last_heartbeat = self.running_secs;
            let _ = tx.send(SchedulerEvent::Heartbeat {
                running_secs: self.running_secs,
            });
        }

        let in_quiet = in_quiet_hours(&cfg.general.quiet_start, &cfg.general.quiet_end);

        // 1) 周期型提醒
        for kind in [
            ReminderKind::Eyes,
            ReminderKind::Stand,
            ReminderKind::Water,
            ReminderKind::Neck,
        ] {
            if !cfg.reminders.is_enabled(kind) {
                continue;
            }
            if in_quiet {
                continue;
            }
            let interval = cfg.reminders.interval_sec(kind).unwrap_or(u64::MAX);
            let last = self.last_fire.get(&kind).copied().unwrap_or(0);
            if self.running_secs.saturating_sub(last) >= interval {
                self.last_fire.insert(kind, self.running_secs);
                let _ = tx.send(SchedulerEvent::Triggered(kind));
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
                let _ = tx.send(SchedulerEvent::Triggered(cur_kind));
            }
        }

        // 3) 大休息：连续工作满则触发（即便勿扰时段也照触发，因为强制）
        if cfg.reminders.enabled.big_break
            && self.big_break_secs >= cfg.reminders.big_break_interval_sec
        {
            self.big_break_secs = 0;
            self.last_fire.insert(ReminderKind::BigBreak, self.running_secs);
            let _ = tx.send(SchedulerEvent::Triggered(ReminderKind::BigBreak));
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
            tx,
        );
        check_time_point(
            ReminderKind::Sleep,
            &cfg.reminders.sleep_time,
            cfg.reminders.enabled.sleep,
            &today,
            now_time,
            &mut self.fired_today,
            tx,
        );

        // 5) 累计型：工作满 8h 提醒下班
        if cfg.reminders.enabled.off_work
            && self.running_secs >= cfg.reminders.off_work_total_sec
            && self.off_work_fired_date.as_deref() != Some(&today)
        {
            self.off_work_fired_date = Some(today.clone());
            let _ = tx.send(SchedulerEvent::Triggered(ReminderKind::OffWork));
        }
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
    tx: &Sender<SchedulerEvent>,
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
            let _ = tx.send(SchedulerEvent::Triggered(kind));
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
        cfg
    }

    #[test]
    fn idle_state_no_tick() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut e = Engine::new();
        let cfg = fast_cfg();
        for _ in 0..100 {
            e.tick(Instant::now(), &cfg, &tx);
        }
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn start_then_eyes_fires_after_interval() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut e = Engine::new();
        let cfg = fast_cfg();
        e.apply(Command::Start, &cfg, &tx);
        // 消费 StateChanged
        let _ = rx.recv();
        for _ in 0..3 {
            e.tick(Instant::now(), &cfg, &tx);
        }
        // 期望至少出现一次 Eyes
        let mut got_eyes = false;
        while let Ok(ev) = rx.try_recv() {
            if let SchedulerEvent::Triggered(ReminderKind::Eyes) = ev {
                got_eyes = true;
            }
        }
        assert!(got_eyes, "未触发 Eyes 提醒");
    }

    #[test]
    fn big_break_independent_of_other_intervals() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut e = Engine::new();
        let cfg = fast_cfg();
        e.apply(Command::Start, &cfg, &tx);
        for _ in 0..cfg.reminders.big_break_interval_sec {
            e.tick(Instant::now(), &cfg, &tx);
        }
        let mut got_big = false;
        while let Ok(ev) = rx.try_recv() {
            if let SchedulerEvent::Triggered(ReminderKind::BigBreak) = ev {
                got_big = true;
            }
        }
        assert!(got_big);
    }

    #[test]
    fn pause_freezes_running_secs() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let mut e = Engine::new();
        let cfg = fast_cfg();
        e.apply(Command::Start, &cfg, &tx);
        for _ in 0..5 {
            e.tick(Instant::now(), &cfg, &tx);
        }
        e.apply(Command::Pause, &cfg, &tx);
        let snapshot = e.running_secs();
        for _ in 0..10 {
            e.tick(Instant::now(), &cfg, &tx);
        }
        assert_eq!(e.running_secs(), snapshot);
    }
}
