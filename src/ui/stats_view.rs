// 统计面板：概要、30 天工作趋势、提醒完成分布

use eframe::egui::{self, RichText};
use egui_plot::{Line, Plot, PlotPoints};

use crate::app::App;
use crate::stats::{fmt_hms, kind_label};
use crate::ui::{theme, widgets};

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::page_header(ui, "统计", "回看近 30 天的工作节奏与健康提醒完成情况。");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("↻").on_hover_text("刷新统计").clicked() {
                        match crate::stats::StatsView::load(&app.db) {
                            Ok(stats) => app.stats = stats,
                            Err(error) => app.show_error(format!("刷新统计失败：{error}")),
                        }
                    }
                });
            });
            ui.add_space(16.0);

            summary(app, ui);
            ui.add_space(18.0);
            work_trend(app, ui);
            ui.add_space(18.0);
            reminder_distribution(app, ui);
            ui.add_space(4.0);
        });
}

fn summary(app: &App, ui: &mut egui::Ui) {
    let (completed_breaks, total_breaks) = app.stats.big_break_today;
    let event_total = app
        .stats
        .kind_dist_30d
        .iter()
        .map(|(_, value)| value)
        .sum::<i64>();

    widgets::section_header(ui, "概览", None);
    ui.add_space(8.0);
    ui.columns(2, |columns| {
        widgets::stat_card(
            &mut columns[0],
            "今",
            fmt_hms(app.stats.today.work_seconds),
            "今日工作",
            theme::INFO,
        );
        widgets::stat_card(
            &mut columns[1],
            "续",
            format!("{} 天", app.stats.streak),
            "连续达标",
            theme::WARN,
        );
        columns[0].add_space(8.0);
        columns[1].add_space(8.0);
        widgets::stat_card(
            &mut columns[0],
            "练",
            format!("{completed_breaks}/{total_breaks}"),
            "今日跟练",
            theme::ACCENT,
        );
        widgets::stat_card(
            &mut columns[1],
            "记",
            event_total.to_string(),
            "30 天完成事件",
            theme::DANGER,
        );
    });
}

fn work_trend(app: &App, ui: &mut egui::Ui) {
    widgets::section_header(ui, "工作时长趋势", Some("近 30 天 · 小时"));
    ui.add_space(8.0);
    if app.stats.last_30.is_empty() {
        empty_state(ui, "还没有历史数据，完成一次工作会话后即可看到趋势。");
        return;
    }

    let labels: Vec<String> = app
        .stats
        .last_30
        .iter()
        .map(|point| point.date.format("%m-%d").to_string())
        .collect();
    let points: PlotPoints = app
        .stats
        .last_30
        .iter()
        .enumerate()
        .map(|(index, point)| [index as f64, point.work_seconds as f64 / 3600.0])
        .collect();

    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, theme::STROKE))
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            Plot::new("work-trend")
                .height(220.0)
                .allow_drag(false)
                .allow_scroll(false)
                .allow_zoom(false)
                .show_axes([true, true])
                .x_axis_formatter(move |mark, _range| {
                    let index = mark.value.round() as isize;
                    if index < 0 {
                        return String::new();
                    }
                    labels.get(index as usize).cloned().unwrap_or_default()
                })
                .show(ui, |plot| {
                    plot.line(
                        Line::new(points)
                            .name("工作小时")
                            .color(theme::INFO)
                            .width(2.0),
                    );
                });
        });
}

fn reminder_distribution(app: &App, ui: &mut egui::Ui) {
    widgets::section_header(ui, "提醒完成分布", Some("近 30 天"));
    ui.add_space(8.0);
    if app.stats.kind_dist_30d.is_empty() {
        empty_state(ui, "还没有提醒完成记录。");
        return;
    }

    let max_value = app
        .stats
        .kind_dist_30d
        .iter()
        .map(|(_, value)| *value)
        .max()
        .unwrap_or(1)
        .max(1);
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, theme::STROKE))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            for (index, (key, value)) in app.stats.kind_dist_30d.iter().enumerate() {
                let color = distribution_color(index);
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [112.0, 24.0],
                        egui::Label::new(
                            RichText::new(kind_label(key)).size(13.0).color(theme::TEXT),
                        ),
                    );
                    let ratio = *value as f32 / max_value as f32;
                    ui.add(
                        egui::ProgressBar::new(ratio.clamp(0.0, 1.0))
                            .desired_width((ui.available_width() - 44.0).max(120.0))
                            .fill(color)
                            .text(""),
                    );
                    ui.label(
                        RichText::new(value.to_string())
                            .monospace()
                            .strong()
                            .color(color),
                    );
                });
                if index + 1 < app.stats.kind_dist_30d.len() {
                    ui.add_space(5.0);
                }
            }
        });
}

fn distribution_color(index: usize) -> egui::Color32 {
    const COLORS: [egui::Color32; 5] = [
        theme::ACCENT,
        theme::INFO,
        theme::WARN,
        theme::DANGER,
        theme::PURPLE,
    ];
    COLORS[index % COLORS.len()]
}

fn empty_state(ui: &mut egui::Ui, message: &str) {
    egui::Frame::none()
        .fill(theme::CARD)
        .rounding(8.0)
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(message).size(13.0).color(theme::TEXT_WEAK));
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribution_palette_cycles_without_panicking() {
        assert_eq!(distribution_color(0), theme::ACCENT);
        assert_eq!(distribution_color(5), theme::ACCENT);
        assert_eq!(distribution_color(9), theme::PURPLE);
    }

    #[test]
    fn empty_state_renders_at_minimum_content_width() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 400.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                empty_state(ui, "暂无数据");
                assert!(ui.min_rect().height() > 20.0);
            });
        });
    }
}
