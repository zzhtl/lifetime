// 主页：专注会话、下一次休息、今日健康行为与最近提醒

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::scheduler::{Command, RunState};
use crate::ui::{theme, widgets};

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::page_header(ui, "今日专注", "专注有时，舒展有度。让提醒自然融入工作节奏。");
                if app.stats.streak > 0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        widgets::status_badge(
                            ui,
                            &format!("连续达标 {} 天", app.stats.streak),
                            theme::WARN,
                        );
                    });
                }
            });
            ui.add_space(16.0);

            focus_panel(app, ui);

            ui.add_space(18.0);
            widgets::section_header(ui, "今日养护", Some("提醒完成后自动累计"));
            ui.add_space(8.0);
            metric_grid(
                ui,
                &[
                    ("水", app.today.water_count.to_string(), "喝水次数", theme::INFO),
                    ("眼", app.today.eye_break_count.to_string(), "护眼次数", theme::ACCENT),
                    ("起", app.today.stand_count.to_string(), "起身次数", theme::WARN),
                    ("颈", app.today.neck_count.to_string(), "颈椎活动", theme::PURPLE),
                    ("番", app.today.pomodoros.to_string(), "番茄休息", theme::DANGER),
                    ("休", app.today.big_breaks.to_string(), "大休息", theme::INFO),
                ],
            );

            ui.add_space(18.0);
            recent_reminder(app, ui);
            ui.add_space(4.0);
        });
}

fn focus_panel(app: &mut App, ui: &mut egui::Ui) {
    let (
        big_break_enabled,
        big_break_interval,
        eyes_enabled,
        eyes_interval,
        pomodoro_enabled,
        pomodoro_focus,
        pomodoro_break,
    ) = {
        let config = app
            .config
            .lock()
            .expect("配置锁不应被持有线程破坏");
        (
            config.reminders.enabled.big_break,
            config.reminders.big_break_interval_sec,
            config.reminders.enabled.eyes,
            config.reminders.eyes_interval_sec,
            config.reminders.enabled.pomodoro,
            config.reminders.pomodoro_focus_sec,
            config.reminders.pomodoro_break_sec,
        )
    };

    let running = app.running_secs;
    let break_left = countdown(big_break_interval, running);
    let eyes_left = countdown(eyes_interval, running);

    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, theme::ACCENT.linear_multiply(0.42)))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.columns(2, |columns| {
                let left = &mut columns[0];
                let (status, hint, color) = match app.run_state {
                    RunState::Idle => ("尚未开始", "启动后将按你的节奏安排健康提醒", theme::TEXT),
                    RunState::Running => ("专注进行中", "计时与提醒正在后台持续运行", theme::ACCENT),
                    RunState::Paused => ("会话已暂停", "暂停期间不会累计工作时间", theme::WARN),
                };
                left.horizontal(|ui| {
                    widgets::status_badge(ui, status, color);
                });
                left.add_space(10.0);
                left.label(
                    RichText::new(crate::stats::fmt_hms(running as i64))
                        .size(34.0)
                        .monospace()
                        .strong()
                        .color(theme::TEXT),
                );
                left.label(RichText::new("本次会话").size(12.5).color(theme::TEXT_WEAK));
                left.add_space(8.0);
                left.add(egui::Label::new(RichText::new(hint).size(12.5).color(theme::TEXT_WEAK)).wrap());
                left.add_space(14.0);
                session_controls(app, left);

                let right = &mut columns[1];
                right.vertical_centered(|ui| {
                    if big_break_enabled {
                        widgets::big_timer(ui, break_left, "距下一次大休息");
                        let elapsed = big_break_interval.saturating_sub(break_left);
                        let ratio = elapsed as f32 / big_break_interval.max(1) as f32;
                        ui.add(
                            egui::ProgressBar::new(ratio.clamp(0.0, 1.0))
                                .desired_width(ui.available_width().min(260.0))
                                .fill(theme::ACCENT)
                                .text(""),
                        );
                    } else {
                        ui.label(RichText::new("大休息未启用").size(18.0).color(theme::TEXT_WEAK));
                    }
                    ui.add_space(9.0);
                    if eyes_enabled {
                        ui.label(
                            RichText::new(format!("护眼提醒  {}", fmt_mmss(eyes_left)))
                                .size(12.5)
                                .color(theme::INFO),
                        );
                    }
                    if pomodoro_enabled {
                        ui.label(
                            RichText::new(format!(
                                "番茄节奏  {} / {} 分钟",
                                pomodoro_focus / 60,
                                pomodoro_break / 60
                            ))
                            .size(12.5)
                            .color(theme::TEXT_WEAK),
                        );
                    }
                });
            });
        });
}

fn metric_grid(ui: &mut egui::Ui, metrics: &[(&str, String, &str, Color32)]) {
    debug_assert_eq!(metrics.len(), 6);
    ui.columns(6, |columns| {
        for (column_ui, (icon, value, label, color)) in columns.iter_mut().zip(metrics) {
            widgets::stat_card(column_ui, icon, value, label, *color);
        }
    });
}

fn session_controls(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| match app.run_state {
        RunState::Idle => {
            if ui
                .add(
                    egui::Button::new(RichText::new("▶  开始专注").strong())
                        .fill(theme::ACCENT.linear_multiply(0.38))
                        .min_size(egui::vec2(132.0, 36.0)),
                )
                .clicked()
            {
                app.start_session_if_needed();
                app.send(Command::Start);
            }
        }
        RunState::Running => {
            if ui
                .add(egui::Button::new("Ⅱ  暂停").min_size(egui::vec2(96.0, 34.0)))
                .clicked()
            {
                app.send(Command::Pause);
            }
            if ui
                .add(egui::Button::new("■  结束").min_size(egui::vec2(84.0, 34.0)))
                .clicked()
            {
                app.send(Command::Stop);
            }
        }
        RunState::Paused => {
            if ui
                .add(
                    egui::Button::new(RichText::new("▶  继续").strong())
                        .fill(theme::ACCENT.linear_multiply(0.32))
                        .min_size(egui::vec2(96.0, 34.0)),
                )
                .clicked()
            {
                app.send(Command::Resume);
            }
            if ui
                .add(egui::Button::new("■  结束").min_size(egui::vec2(84.0, 34.0)))
                .clicked()
            {
                app.send(Command::Stop);
            }
        }
    });
}

fn recent_reminder(app: &App, ui: &mut egui::Ui) {
    widgets::section_header(ui, "最近提醒", None);
    ui.add_space(8.0);
    egui::Frame::none()
        .fill(theme::CARD_ALT)
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match app.last_reminder {
                Some((kind, timestamp)) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("●").color(theme::ACCENT));
                        ui.label(RichText::new(kind.label()).strong());
                        ui.label(
                            RichText::new(timestamp.format("%H:%M:%S").to_string())
                                .monospace()
                                .color(theme::TEXT_WEAK),
                        );
                        ui.label(RichText::new(kind.brief()).size(12.5).color(theme::TEXT_WEAK));
                    });
                }
                None => {
                    ui.label(
                        RichText::new("今天还没有提醒记录，开始专注后会在这里显示。")
                            .size(12.5)
                            .color(theme::TEXT_WEAK),
                    );
                }
            }
        });
}

fn countdown(interval: u64, running: u64) -> u64 {
    if interval == 0 {
        return 0;
    }
    let elapsed = running % interval;
    if elapsed == 0 && running > 0 {
        0
    } else {
        interval.saturating_sub(elapsed)
    }
}

fn fmt_mmss(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_handles_start_boundary_and_zero_interval() {
        assert_eq!(countdown(1200, 0), 1200);
        assert_eq!(countdown(1200, 1), 1199);
        assert_eq!(countdown(1200, 1200), 0);
        assert_eq!(countdown(0, 100), 0);
    }

    #[test]
    fn metric_grid_renders_six_cards_in_one_row() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1072.0, 500.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let right_edge = ui.max_rect().right();
                let response = ui
                    .scope(|ui| {
                        metric_grid(
                            ui,
                            &[
                                ("水", "1".into(), "喝水", theme::INFO),
                                ("眼", "2".into(), "护眼", theme::ACCENT),
                                ("起", "3".into(), "起身", theme::WARN),
                                ("颈", "4".into(), "颈椎", theme::PURPLE),
                                ("番", "5".into(), "番茄", theme::DANGER),
                                ("休", "6".into(), "休息", theme::INFO),
                            ],
                        );
                    })
                    .response;
                assert!(response.rect.right() <= right_edge + 1.0);
                assert!(response.rect.height() < 100.0);
            });
        });
    }
}
