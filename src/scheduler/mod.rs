// 调度核心
// 跑在独立线程上，每秒 tick 一次；
// 通过 crossbeam channel 与 UI 线程双向通信

mod event;
mod rules;

pub use event::*;
pub use rules::Engine;

use anyhow::Result;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::notify::Notifier;
use crate::reminders::{Intensity, ReminderKind};
use crate::tips::Library;

/// 通知正文轮换缓存：类目 key → 上次用过的标题（避免紧邻重复）
type TipRotation = HashMap<&'static str, String>;

/// 调度器对外句柄
pub struct SchedulerHandle {
    pub cmd_tx: Sender<Command>,
    pub evt_rx: Receiver<SchedulerEvent>,
}

/// 启动调度器线程
pub fn spawn(config: Arc<Mutex<Config>>) -> Result<SchedulerHandle> {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<Command>();
    let (evt_tx, evt_rx) = crossbeam_channel::unbounded::<SchedulerEvent>();

    let cfg_for_thread = Arc::clone(&config);
    thread::Builder::new()
        .name("lifetime-scheduler".to_string())
        .spawn(move || run_loop(cfg_for_thread, cmd_rx, evt_tx))?;

    Ok(SchedulerHandle { cmd_tx, evt_rx })
}

fn run_loop(
    config: Arc<Mutex<Config>>,
    cmd_rx: Receiver<Command>,
    evt_tx: Sender<SchedulerEvent>,
) {
    let mut engine = Engine::new();
    // 通知与音效在调度线程内构造并发出，这样窗口最小化/失焦时也能可靠提醒，
    // 不再依赖 UI 线程的重绘循环（rodio 的音频流非 Send，必须留在本线程）。
    let notifier = Notifier::new();
    // 健康库在调度线程内一次解析，用于轮换桌面通知正文（具体动作/小贴士）
    let tips = Library::load().unwrap_or_default();
    let mut rotation: TipRotation = HashMap::new();
    let tick = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    loop {
        // 等待控制指令，最多阻塞到下一个 tick 边界，从而命令可即时响应
        let timeout = tick.saturating_sub(last_tick.elapsed());
        match cmd_rx.recv_timeout(timeout) {
            Ok(cmd) => {
                if process_command(cmd, &mut engine, &config, &notifier, &evt_tx, &tips, &mut rotation) {
                    return; // 收到 Quit
                }
                // 排空同一时刻积压的其余命令
                while let Ok(cmd) = cmd_rx.try_recv() {
                    if process_command(cmd, &mut engine, &config, &notifier, &evt_tx, &tips, &mut rotation) {
                        return;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        // 按真实流逝时间逐秒推进，避免唤醒/处理耗时造成漂移或丢秒
        while last_tick.elapsed() >= tick {
            last_tick += tick;
            let cfg = config.lock().unwrap().clone();
            let out = engine.tick(Instant::now(), &cfg);
            if let Some(running_secs) = out.heartbeat {
                let _ = evt_tx.send(SchedulerEvent::Heartbeat { running_secs });
            }
            for kind in out.triggered {
                fire(kind, &cfg, &notifier, &evt_tx, &tips, &mut rotation);
            }
        }
    }
}

/// 处理一条控制指令，返回 true 表示需要退出线程
fn process_command(
    cmd: Command,
    engine: &mut Engine,
    config: &Arc<Mutex<Config>>,
    notifier: &Notifier,
    evt_tx: &Sender<SchedulerEvent>,
    tips: &Library,
    rotation: &mut TipRotation,
) -> bool {
    let cfg = config.lock().unwrap().clone();
    match cmd {
        Command::Quit => return true,
        // 试听：仅出声
        Command::TestSound => notifier.beep(Intensity::Medium, cfg.general.volume),
        // 测试通知：强制发一条示例桌面通知 + 声音，用于即时验证系统通知是否可用
        Command::TestNotify => notifier.dispatch(
            ReminderKind::Eyes,
            ReminderKind::Eyes.brief(),
            true,
            true,
            cfg.general.volume,
        ),
        other => {
            let out = engine.apply(other, &cfg);
            if let Some(state) = out.state_changed {
                let _ = evt_tx.send(SchedulerEvent::StateChanged(state));
            }
            if let Some(kind) = out.triggered {
                fire(kind, &cfg, notifier, evt_tx, tips, rotation);
            }
        }
    }
    false
}

/// 一次提醒触发：在调度线程直接发出桌面通知 / 声音，同时通知 UI 线程更新统计与模态
fn fire(
    kind: ReminderKind,
    cfg: &Config,
    notifier: &Notifier,
    evt_tx: &Sender<SchedulerEvent>,
    tips: &Library,
    rotation: &mut TipRotation,
) {
    // 交给 UI 线程：DB 记录、当日计数、大休息模态窗
    let _ = evt_tx.send(SchedulerEvent::Triggered(kind));

    let desktop_on = cfg.general.desktop_notify;
    let sound_on = cfg.general.sound_enabled;
    let volume = cfg.general.volume;
    if kind == ReminderKind::BigBreak {
        // 大休息：桌面通知留给模态窗，避免双重打扰；这里只出声
        if sound_on {
            notifier.beep(kind.intensity(), volume);
        }
    } else {
        let body = pick_notify_body(tips, rotation, kind);
        notifier.dispatch(kind, &body, desktop_on, sound_on, volume);
    }
}

/// 为提醒挑一条轮换正文：从对应类目随机取一条 tip（标题 + 首步），避免与上次重复；
/// 没有类目或库为空时退回 kind.brief()，保证总有正文。
fn pick_notify_body(tips: &Library, rotation: &mut TipRotation, kind: ReminderKind) -> String {
    let Some(cat) = kind.tip_category() else {
        return kind.brief().to_string();
    };
    let mut list = tips.office_break_by_category_key(cat);
    if list.is_empty() {
        list = tips.by_category_key(cat);
    }
    if list.is_empty() {
        return kind.brief().to_string();
    }
    let mut rng = rand::thread_rng();
    let prev = rotation.get(cat).cloned();
    // 类目内多于一条时，尽量避开上次用过的标题
    let chosen = if list.len() > 1 {
        loop {
            let t = *list.choose(&mut rng).unwrap();
            if prev.as_deref() != Some(t.title.as_str()) {
                break t;
            }
        }
    } else {
        *list.choose(&mut rng).unwrap()
    };
    rotation.insert(cat, chosen.title.clone());
    match chosen.steps.first() {
        Some(step) => format!("{} —— {}", chosen.title, step),
        None => chosen.title.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    /// 回归测试：调度线程必须在“没有任何 UI/update 循环”的情况下也能触发提醒。
    /// 这等价于主窗口被最小化/失焦的真实场景——也正是此前“到点无通知”的根因。
    #[test]
    fn scheduler_fires_without_ui() {
        let mut cfg = Config::default();
        cfg.reminders.eyes_interval_sec = 1;
        cfg.general.quiet_start = "00:00".into();
        cfg.general.quiet_end = "00:00".into();
        // 关闭错开，验证"到点即触发"的基础链路
        cfg.general.min_notify_gap_sec = 0;
        // 测试中不弹真实系统通知、不出声，只验证触发链路
        cfg.general.desktop_notify = false;
        cfg.general.sound_enabled = false;

        let handle = spawn(Arc::new(Mutex::new(cfg))).expect("spawn scheduler");
        handle.cmd_tx.send(Command::Start).expect("send start");

        let deadline = Instant::now() + Duration::from_secs(4);
        let mut got_eyes = false;
        while Instant::now() < deadline {
            if let Ok(SchedulerEvent::Triggered(ReminderKind::Eyes)) =
                handle.evt_rx.recv_timeout(Duration::from_millis(200))
            {
                got_eyes = true;
                break;
            }
        }
        let _ = handle.cmd_tx.send(Command::Quit);
        assert!(got_eyes, "调度线程未在无 UI 情况下触发护眼");
    }

    #[test]
    fn rest_notifications_prefer_office_break_tips() {
        let tips = Library::load().unwrap();
        let mut rotation: TipRotation = HashMap::new();

        for kind in [
            ReminderKind::Eyes,
            ReminderKind::Stand,
            ReminderKind::Neck,
            ReminderKind::PomodoroBreak,
        ] {
            for _ in 0..20 {
                let body = pick_notify_body(&tips, &mut rotation, kind);
                let title = body.split(" —— ").next().unwrap_or(body.as_str());
                let tip = tips
                    .all()
                    .iter()
                    .find(|t| t.title == title)
                    .expect("通知正文应来自知识库");
                assert!(
                    tip.office_break,
                    "{kind:?} 不应抽到非办公室动作: {}",
                    tip.title
                );
            }
        }
    }
}
