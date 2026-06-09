// 主面板（Dashboard）
// 展示当前阶段、倒计时、当日统计、最近提醒

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::reminders::ReminderKind;
use crate::scheduler::Command;
use crate::ui::widgets;

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("📊 今日工作概览");
    if app.stats.streak > 0 {
        ui.label(
            RichText::new(format!("🔥 已连续达标 {} 天，继续保持～", app.stats.streak))
                .size(13.0)
                .color(Color32::from_rgb(230, 150, 90)),
        );
    }
    ui.add_space(8.0);

    // 当前阶段与下一次大休息预计
    let (big_break_interval, eyes_interval, pomodoro_focus) = {
        let cfg = app.config.lock().unwrap();
        (
            cfg.reminders.big_break_interval_sec,
            cfg.reminders.eyes_interval_sec,
            cfg.reminders.pomodoro_focus_sec,
        )
    };
    let running = app.running_secs;
    let big_break_left = big_break_interval.saturating_sub(running % big_break_interval.max(1));
    let eyes_left = eyes_interval.saturating_sub(running % eyes_interval.max(1));

    egui::Frame::group(ui.style()).rounding(8.0).inner_margin(16.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let phase_text = match app.run_state {
                    crate::scheduler::RunState::Idle => "未开始 · 点击「▶ 开始」启动工作会话",
                    crate::scheduler::RunState::Running => "🎯 专注工作中",
                    crate::scheduler::RunState::Paused => "⏸ 已暂停",
                };
                ui.label(RichText::new(phase_text).size(16.0));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("本次会话已工作 {}", crate::stats::fmt_hms(running as i64)))
                        .color(Color32::LIGHT_BLUE),
                );
            });

            ui.add_space(40.0);

            ui.vertical(|ui| {
                widgets::big_timer(
                    ui,
                    big_break_left,
                    "距下一次大休息",
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "护眼倒计时 {}",
                        fmt_mmss(eyes_left)
                    ))
                    .weak(),
                );
                ui.label(
                    RichText::new(format!(
                        "番茄钟周期 {} / 休息 {}",
                        fmt_min(pomodoro_focus),
                        fmt_min({
                            let cfg = app.config.lock().unwrap();
                            cfg.reminders.pomodoro_break_sec
                        })
                    ))
                    .weak(),
                );
            });
        });
    });

    ui.add_space(16.0);
    ui.label(RichText::new("📅 今日累计").size(14.0).strong());
    ui.add_space(4.0);

    // 自动折行：窗口变窄时卡片换行，避免被面板边缘裁切
    ui.horizontal_wrapped(|ui| {
        widgets::stat_card(ui, "💧", format!("{}", app.today.water_count), "喝水次数", Color32::LIGHT_BLUE);
        widgets::stat_card(ui, "👁", format!("{}", app.today.eye_break_count), "护眼次数", Color32::from_rgb(120, 200, 120));
        widgets::stat_card(ui, "🚶", format!("{}", app.today.stand_count), "起身次数", Color32::from_rgb(220, 180, 80));
        widgets::stat_card(ui, "🍅", format!("{}", app.today.pomodoros), "番茄钟", Color32::from_rgb(220, 100, 80));
        widgets::stat_card(ui, "🦴", format!("{}", app.today.neck_count), "颈椎活动", Color32::from_rgb(180, 140, 220));
        widgets::stat_card(ui, "🛌", format!("{}", app.today.big_breaks), "大休息次数", Color32::from_rgb(120, 180, 200));
        widgets::stat_card(ui, "⏱", crate::stats::fmt_hms(app.today.work_seconds), "今日工作", Color32::LIGHT_GRAY);
    });

    ui.add_space(16.0);

    // 最近一次提醒
    if let Some((kind, ts)) = app.last_reminder {
        ui.separator();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("最近提醒：").weak());
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    kind.label(),
                    ts.format("%H:%M:%S")
                ))
                .strong(),
            );
        });
    }

    ui.add_space(12.0);
    ui.collapsing("🛠 手动触发（调试）", |ui| {
        ui.horizontal_wrapped(|ui| {
            for k in ReminderKind::all() {
                if ui.small_button(k.label()).clicked() {
                    app.send(Command::TriggerNow(*k));
                }
            }
        });
    });
}

fn fmt_mmss(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{m:02}:{s:02}")
}

fn fmt_min(secs: u64) -> String {
    format!("{} 分钟", secs / 60)
}
