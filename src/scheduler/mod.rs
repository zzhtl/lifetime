// 调度核心
// 跑在独立线程上，每秒 tick 一次；
// 通过 crossbeam channel 与 UI 线程双向通信

mod event;
mod rules;

pub use event::*;
pub use rules::Engine;

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;

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
    let tick = Duration::from_secs(1);

    loop {
        // 优先消费控制指令
        while let Ok(cmd) = cmd_rx.try_recv() {
            let exit = matches!(cmd, Command::Quit);
            let cfg = config.lock().unwrap().clone();
            engine.apply(cmd, &cfg, &evt_tx);
            if exit {
                return;
            }
        }

        let cfg = config.lock().unwrap().clone();
        engine.tick(Instant::now(), &cfg, &evt_tx);

        thread::sleep(tick);
    }
}
