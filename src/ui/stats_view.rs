// 统计面板
// 用 egui_plot 画近 30 天工作时长 + 提醒分布

use eframe::egui::{self, Color32, RichText};
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};

use crate::app::App;
use crate::stats::{fmt_hms, kind_label};

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("📈 统计");
    ui.add_space(8.0);

    if ui.button("🔄 刷新").clicked() {
        if let Ok(s) = crate::stats::StatsView::load(&app.db) {
            app.stats = s;
        }
    }
    ui.add_space(8.0);

    // 概要卡片
    ui.horizontal(|ui| {
        summary_box(ui, "📅 今日工作", &fmt_hms(app.stats.today.work_seconds), Color32::LIGHT_BLUE);
        summary_box(
            ui,
            "🗓 近 30 天事件总数",
            &format!(
                "{}",
                app.stats.kind_dist_30d.iter().map(|(_, v)| v).sum::<i64>()
            ),
            Color32::LIGHT_GREEN,
        );
        summary_box(
            ui,
            "📈 有数据天数",
            &format!("{}", app.stats.last_30.len()),
            Color32::from_rgb(220, 180, 80),
        );
    });

    ui.add_space(16.0);
    ui.label(RichText::new("近 30 天工作时长（小时）").strong());
    Plot::new("work-trend")
        .height(200.0)
        .show_axes([true, true])
        .show(ui, |plot| {
            let points: PlotPoints = app
                .stats
                .last_30
                .iter()
                .enumerate()
                .map(|(i, d)| [i as f64, d.work_seconds as f64 / 3600.0])
                .collect();
            plot.line(Line::new(points).name("工作小时").color(Color32::LIGHT_BLUE));
        });

    ui.add_space(16.0);
    ui.label(RichText::new("近 30 天各类提醒完成次数").strong());
    Plot::new("kind-dist")
        .height(220.0)
        .show_axes([true, true])
        .show(ui, |plot| {
            let bars: Vec<Bar> = app
                .stats
                .kind_dist_30d
                .iter()
                .enumerate()
                .map(|(i, (key, v))| {
                    Bar::new(i as f64, *v as f64).name(kind_label(key))
                })
                .collect();
            plot.bar_chart(BarChart::new(bars).color(Color32::from_rgb(220, 100, 80)));
        });

    ui.add_space(8.0);

    if app.stats.last_30.is_empty() {
        ui.label(
            RichText::new("还没有历史数据，先开始一个工作会话吧～")
                .color(Color32::LIGHT_GRAY)
                .italics(),
        );
    }
}

fn summary_box(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::group(ui.style())
        .rounding(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(12.0).weak());
                ui.label(RichText::new(value).size(18.0).strong().color(color));
            });
        });
}
