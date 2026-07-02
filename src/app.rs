// AppState + eframe::App 主循环

use anyhow::Result;
use chrono::Local;
use eframe::egui::{self, Color32, RichText};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::breathing::BreathingState;
use crate::config::Config;
use crate::db::{self, Db, DailySummary, ReminderAction};
use crate::practices::PracticeLibrary;
use crate::reminders::ReminderKind;
use crate::scheduler::{self, Command, RunState, SchedulerEvent, SchedulerHandle};
use crate::stats::StatsView;
use crate::tips::{Library, RoutineSegment, TipCategory};
use crate::ui::{self, View};

/// 模态休息窗状态（大休息「分段跟练」）
pub struct BreakState {
    pub kind: ReminderKind,
    /// 整条路线总时长（进度条用）
    pub total_secs: u64,
    /// 整体剩余秒数
    pub remaining: u64,
    pub skip_available_in: u64,
    /// 分段动作清单
    pub segments: Vec<RoutineSegment>,
    /// 当前进行到第几节
    pub seg_index: usize,
    /// 当前小节剩余秒数（大字倒计时）
    pub seg_remaining: u64,
}

/// 修炼成长运行时状态（内存缓存，避免每帧查库）
#[derive(Default)]
pub struct CultivationState {
    /// 累计修为值（打卡数）
    pub points: i64,
    /// 今日已打卡的功法标题
    pub today_logged: Vec<String>,
}

pub struct App {
    pub config: Arc<Mutex<Config>>,
    pub db: Db,
    pub tips: Library,
    pub practices: PracticeLibrary,
    pub sched: SchedulerHandle,

    pub view: View,
    pub run_state: RunState,
    pub running_secs: u64,
    pub session_id: Option<i64>,
    pub today: DailySummary,
    pub stats: StatsView,
    /// 修炼打卡成长状态（修为值 + 今日已打卡）
    pub cultivation: CultivationState,

    /// 呼吸引导练习台运行时状态
    pub breathing: BreathingState,
    /// 今日呼吸练习次数
    pub breathing_count: i64,
    /// 今日呼吸练习总秒数
    pub breathing_secs: i64,
    /// 连续呼吸练习天数
    pub breathing_streak: i64,

    pub pending_break: Option<BreakState>,
    /// 上一次大休息跟练用过的动作标题，用于下次避免重复
    pub last_break_titles: Vec<String>,
    /// 最近一次轻量提醒，用于在 dashboard 显示
    pub last_reminder: Option<(ReminderKind, chrono::DateTime<chrono::Local>)>,
    pub error_msg: Option<String>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, cfg: Config) -> Result<Self> {
        let db = db::open(&cfg.paths.db_file)?;
        let tips = Library::load()?;
        let practices = PracticeLibrary::load()?;
        // 如果上次崩溃时还有未结束会话，先收尾
        if let Some(sid) = db::last_open_session(&db)? {
            let _ = db::end_session(&db, sid, 0);
        }
        let config = Arc::new(Mutex::new(cfg));
        let sched = scheduler::spawn(Arc::clone(&config))?;
        let today = db::get_today(&db)?;
        let stats = StatsView::load(&db).unwrap_or_default();
        let cultivation = CultivationState {
            points: db::practice_points(&db).unwrap_or(0),
            today_logged: db::practice_logged_today(&db).unwrap_or_default(),
        };
        let (breathing_count, breathing_secs) = db::breathing_today(&db).unwrap_or((0, 0));
        let breathing_streak = db::breathing_streak(&db).unwrap_or(0);
        Ok(Self {
            config,
            db,
            tips,
            practices,
            sched,
            view: View::Dashboard,
            run_state: RunState::Idle,
            running_secs: 0,
            session_id: None,
            today,
            stats,
            cultivation,
            breathing: BreathingState::default(),
            breathing_count,
            breathing_secs,
            breathing_streak,
            pending_break: None,
            last_break_titles: Vec::new(),
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
        // 桌面通知与声音已由调度线程发出（保证窗口最小化/失焦时也能提醒）；
        // UI 线程这里负责入库统计，以及大休息的全屏模态窗。
        if kind == ReminderKind::BigBreak {
            // 大休息：触发时只弹模态；完成/跳过在关窗时记账（用于"跟练完成度"）
            self.open_big_break();
        } else {
            // 其余提醒到点即视为完成
            let _ = db::record_event(&self.db, self.session_id, kind, ReminderAction::Completed);
            match kind {
                ReminderKind::Water => self.today.water_count += 1,
                ReminderKind::Stand => self.today.stand_count += 1,
                ReminderKind::Eyes => self.today.eye_break_count += 1,
                ReminderKind::Neck => self.today.neck_count += 1,
                ReminderKind::PomodoroBreak => self.today.pomodoros += 1,
                _ => {}
            }
            let _ = db::upsert_today(&self.db, &self.today);
        }

        self.last_reminder = Some((kind, Local::now()));
    }

    /// 构造一次大休息「分段跟练」并弹出模态窗
    fn open_big_break(&mut self) {
        let (dur, skip_cd) = {
            let cfg = self.config.lock().unwrap();
            (
                cfg.reminders.big_break_duration_sec,
                cfg.general.skip_cooldown_sec,
            )
        };
        let mut segments = self.tips.build_break_routine(dur, &self.last_break_titles);
        if segments.is_empty() {
            // 兜底：知识库异常为空时也保证有一节内容
            segments.push(RoutineSegment {
                category: TipCategory::Breathing,
                title: ReminderKind::BigBreak.label().to_string(),
                steps: vec![ReminderKind::BigBreak.brief().to_string()],
                benefit: String::new(),
                seconds: dur.max(1),
            });
        }
        self.last_break_titles = segments.iter().map(|s| s.title.clone()).collect();
        let total = segments.iter().map(|s| s.seconds).sum::<u64>().max(1);
        let seg_remaining = segments[0].seconds;
        self.pending_break = Some(BreakState {
            kind: ReminderKind::BigBreak,
            total_secs: total,
            remaining: total,
            skip_available_in: skip_cd,
            segments,
            seg_index: 0,
            seg_remaining,
        });
    }

    /// 记录一次大休息的结果（完成或跳过），用于"今日跟练完成度"统计
    pub fn record_big_break(&mut self, completed: bool) {
        let action = if completed {
            ReminderAction::Completed
        } else {
            ReminderAction::Skipped
        };
        let _ = db::record_event(&self.db, self.session_id, ReminderKind::BigBreak, action);
        if completed {
            self.today.big_breaks += 1;
            let _ = db::upsert_today(&self.db, &self.today);
        }
    }

    /// 记一次修炼打卡：写库 + 更新内存缓存（同日同功法幂等，不重复计数）。
    pub fn log_practice(&mut self, category: &str, title: String) {
        if self.cultivation.today_logged.contains(&title) {
            return;
        }
        if db::log_practice(&self.db, category, &title).is_ok() {
            self.cultivation.points += 1;
            self.cultivation.today_logged.push(title);
        }
    }

    /// 完成一次呼吸练习：写独立明细 + 计入修为（当日 +1，幂等）+ 刷新缓存。
    /// cycles 为本次完成轮数，duration_secs 为本次时长；轮数为 0 视为无效不记。
    pub fn finish_breathing(&mut self, pattern_key: &str, cycles: u32, duration_secs: u32) {
        if cycles == 0 {
            return;
        }
        let _ = db::log_breathing(&self.db, pattern_key, cycles, duration_secs);
        // 计入修为境界：与全站打卡同一体系，同名当日幂等，最多 +1/天
        self.log_practice("breathing", "今日呼吸练习".to_string());
        self.refresh_breathing_stats();
    }

    fn refresh_breathing_stats(&mut self) {
        let (count, secs) = db::breathing_today(&self.db).unwrap_or((0, 0));
        self.breathing_count = count;
        self.breathing_secs = secs;
        self.breathing_streak = db::breathing_streak(&self.db).unwrap_or(0);
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
                    // 当前小节倒计时；归零则自动进入下一节（跟练逐节推进）
                    if b.seg_remaining > 0 {
                        b.seg_remaining -= 1;
                    }
                    if b.seg_remaining == 0 && b.seg_index + 1 < b.segments.len() {
                        b.seg_index += 1;
                        b.seg_remaining = b.segments[b.seg_index].seconds;
                    }
                    acc -= 1.0;
                }
                ctx.memory_mut(|m| m.data.insert_temp(key, acc));
                ctx.request_repaint();
            } else {
                let kind = b.kind;
                self.pending_break = None;
                // 倒计时自然走完视为完成
                self.record_big_break(true);
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
            View::Breathing => ui::breathing::render(self, ui),
            View::Library => ui::library::render(self, ui),
            View::Practice => ui::practice::render(self, ui),
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
