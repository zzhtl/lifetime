// AppState + eframe::App 主循环

use anyhow::Result;
use chrono::Local;
use eframe::egui::{self, Color32, RichText};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::Config;
use crate::db::{self, Db, DailySummary, ReminderAction};
use crate::reminders::ReminderKind;
use crate::scheduler::{self, Command, RunState, SchedulerEvent, SchedulerHandle};
use crate::stats::StatsView;
use crate::tips::Library;
use crate::ui::{self, View};

/// 模态休息窗状态
pub struct BreakState {
    pub kind: ReminderKind,
    pub total_secs: u64,
    pub remaining: u64,
    pub skip_available_in: u64,
    pub tip_title: String,
    pub tip_steps: Vec<String>,
    pub tip_benefit: String,
}

pub struct App {
    pub config: Arc<Mutex<Config>>,
    pub db: Db,
    pub tips: Library,
    pub sched: SchedulerHandle,

    pub view: View,
    pub run_state: RunState,
    pub running_secs: u64,
    pub session_id: Option<i64>,
    pub today: DailySummary,
    pub stats: StatsView,

    pub pending_break: Option<BreakState>,
    /// 最近一次轻量提醒，用于在 dashboard 显示
    pub last_reminder: Option<(ReminderKind, chrono::DateTime<chrono::Local>)>,
    pub error_msg: Option<String>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, cfg: Config) -> Result<Self> {
        let db = db::open(&cfg.paths.db_file)?;
        let tips = Library::load()?;
        // 如果上次崩溃时还有未结束会话，先收尾
        if let Some(sid) = db::last_open_session(&db)? {
            let _ = db::end_session(&db, sid, 0);
        }
        let config = Arc::new(Mutex::new(cfg));
        let sched = scheduler::spawn(Arc::clone(&config))?;
        let today = db::get_today(&db)?;
        let stats = StatsView::load(&db).unwrap_or_default();
        Ok(Self {
            config,
            db,
            tips,
            sched,
            view: View::Dashboard,
            run_state: RunState::Idle,
            running_secs: 0,
            session_id: None,
            today,
            stats,
            pending_break: None,
            last_reminder: None,
            error_msg: None,
        })
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        // 收集后再批处理，避免 borrow 冲突
        let mut events = Vec::new();
        while let Ok(ev) = self.sched.evt_rx.try_recv() {
            events.push(ev);
        }
        for ev in events {
            match ev {
                SchedulerEvent::Heartbeat { running_secs, .. } => {
                    self.running_secs = running_secs;
                    // 顺便累计当日工作秒数
                    self.today.work_seconds = self.running_secs as i64;
                    let _ = db::upsert_today(&self.db, &self.today);
                }
                SchedulerEvent::StateChanged(state) => {
                    self.run_state = state;
                    if state == RunState::Idle {
                        if let Some(sid) = self.session_id.take() {
                            let _ = db::end_session(&self.db, sid, self.running_secs as i64);
                        }
                        self.running_secs = 0;
                    }
                    ctx.request_repaint();
                }
                SchedulerEvent::Triggered(kind) => {
                    self.handle_reminder(kind);
                    ctx.request_repaint();
                }
            }
        }
    }

    fn handle_reminder(&mut self, kind: ReminderKind) {
        // 入库记录（默认 completed；用户在模态窗里有跳过/暂缓选项时再覆写）
        let _ = db::record_event(&self.db, self.session_id, kind, ReminderAction::Completed);
        // 更新当日计数
        match kind {
            ReminderKind::Water => self.today.water_count += 1,
            ReminderKind::Stand => self.today.stand_count += 1,
            ReminderKind::Eyes => self.today.eye_break_count += 1,
            ReminderKind::Neck => self.today.neck_count += 1,
            ReminderKind::PomodoroBreak => self.today.pomodoros += 1,
            ReminderKind::BigBreak => self.today.big_breaks += 1,
            _ => {}
        }
        let _ = db::upsert_today(&self.db, &self.today);

        // 桌面通知与声音已由调度线程发出（保证窗口最小化/失焦时也能提醒）；
        // UI 线程这里只负责大休息的全屏模态窗。
        if kind == ReminderKind::BigBreak {
            let (big_break_secs, skip_cd) = {
                let cfg = self.config.lock().unwrap();
                (
                    cfg.reminders.big_break_duration_sec,
                    cfg.general.skip_cooldown_sec,
                )
            };
            let tip = kind
                .tip_category()
                .and_then(|c| self.tips.random_for_category(c));
            self.pending_break = Some(BreakState {
                kind,
                total_secs: big_break_secs,
                remaining: big_break_secs,
                skip_available_in: skip_cd,
                tip_title: tip.map(|t| t.title.clone()).unwrap_or_else(|| kind.label().to_string()),
                tip_steps: tip.map(|t| t.steps.clone()).unwrap_or_default(),
                tip_benefit: tip.map(|t| t.benefit.clone()).unwrap_or_default(),
            });
        }

        self.last_reminder = Some((kind, Local::now()));
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.sched.cmd_tx.send(cmd);
    }

    /// 切换 Start/Pause/Resume/Stop 按钮组逻辑
    pub fn start_session_if_needed(&mut self) {
        if self.session_id.is_none() {
            if let Ok(sid) = db::start_session(&self.db) {
                self.session_id = Some(sid);
            }
        }
    }

    fn refresh_stats(&mut self) {
        if let Ok(s) = StatsView::load(&self.db) {
            self.stats = s;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1s 节奏刷新（也响应外部事件触发的 repaint）
        ctx.request_repaint_after(Duration::from_millis(1000));

        self.drain_events(ctx);

        // 倒计时模态窗：用 ctx.memory 累加 dt，每满 1 秒推进一次
        if let Some(ref mut b) = self.pending_break {
            if b.remaining > 0 {
                let dt = ctx.input(|i| i.stable_dt).max(0.0) as f64;
                let key = egui::Id::new("break_dt_acc");
                let mut acc: f64 = ctx.memory(|m| m.data.get_temp(key).unwrap_or_default());
                acc += dt;
                while acc >= 1.0 {
                    if b.remaining > 0 {
                        b.remaining -= 1;
                    }
                    if b.skip_available_in > 0 {
                        b.skip_available_in -= 1;
                    }
                    acc -= 1.0;
                }
                ctx.memory_mut(|m| m.data.insert_temp(key, acc));
                ctx.request_repaint();
            } else {
                let kind = b.kind;
                self.pending_break = None;
                self.send(Command::AcknowledgeBreak(kind));
            }
        }

        // 顶栏
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("🌱 Lifetime · 健康助手");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let today_label = format!(
                        "今日已工作 {}",
                        crate::stats::fmt_hms(self.today.work_seconds)
                    );
                    ui.label(RichText::new(today_label).color(Color32::LIGHT_GREEN));
                    ui.separator();
                    self.controls(ui);
                });
            });
            ui.add_space(4.0);
        });

        // 侧栏
        egui::SidePanel::left("sidebar").resizable(false).default_width(140.0).show(ctx, |ui| {
            ui.add_space(8.0);
            for v in View::all() {
                let selected = self.view == *v;
                let txt = RichText::new(format!("{}  {}", v.icon(), v.label())).size(14.0);
                let btn = ui.selectable_label(selected, txt);
                if btn.clicked() {
                    self.view = *v;
                    if matches!(v, View::Stats) {
                        self.refresh_stats();
                    }
                }
            }
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            let phase = match self.run_state {
                RunState::Idle => "🟦 未开始",
                RunState::Running => "🟢 工作中",
                RunState::Paused => "🟡 已暂停",
            };
            ui.label(RichText::new(phase).size(13.0));
        });

        // 中央内容
        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::Dashboard => ui::dashboard::render(self, ui),
            View::Library => ui::library::render(self, ui),
            View::Stats => ui::stats_view::render(self, ui),
            View::Settings => ui::settings::render(self, ui),
            View::About => render_about(ui),
        });

        // 错误提示
        if let Some(err) = self.error_msg.clone() {
            egui::Window::new("提示")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(err);
                    if ui.button("知道了").clicked() {
                        self.error_msg = None;
                    }
                });
        }

        // 模态休息窗（多视口）
        if self.pending_break.is_some() {
            ui::break_window::render_break_viewport(self, ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(sid) = self.session_id.take() {
            let _ = db::end_session(&self.db, sid, self.running_secs as i64);
        }
        let _ = self.sched.cmd_tx.send(Command::Quit);
    }
}

impl App {
    fn controls(&mut self, ui: &mut egui::Ui) {
        match self.run_state {
            RunState::Idle => {
                if ui.button(RichText::new("▶ 开始").strong()).clicked() {
                    self.start_session_if_needed();
                    self.send(Command::Start);
                }
            }
            RunState::Running => {
                if ui.button("⏸ 暂停").clicked() {
                    self.send(Command::Pause);
                }
                if ui.button("⏹ 结束").clicked() {
                    self.send(Command::Stop);
                }
            }
            RunState::Paused => {
                if ui.button("▶ 继续").clicked() {
                    self.send(Command::Resume);
                }
                if ui.button("⏹ 结束").clicked() {
                    self.send(Command::Stop);
                }
            }
        }
    }
}

fn render_about(ui: &mut egui::Ui) {
    ui.heading("关于 Lifetime");
    ui.add_space(8.0);
    ui.label("一款面向程序员/久坐人群的科学健康提醒工具。");
    ui.add_space(8.0);
    ui.label("• 20-20-20 护眼、起身、喝水、颈椎、番茄钟 / 大休息");
    ui.label("• 健康知识库（9 大类，含具体动作步骤与好处）");
    ui.label("• SQLite 长期统计，跨平台（Linux/macOS/Windows）");
    ui.add_space(12.0);
    ui.label("数据完全本地保存，不联网。");
}
