// 通用 UI 小部件

use eframe::egui::{self, Color32, RichText, Stroke, Ui};

/// 小标签（圆角胶囊），用于时长/频率/分类等元信息。
pub fn badge(ui: &mut Ui, text: impl Into<String>, color: Color32) {
    egui::Frame::none()
        .fill(color.linear_multiply(0.18))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).color(color));
        });
}

/// 统计卡片（图标 + 数字 + 标题）
pub fn stat_card(ui: &mut Ui, icon: &str, value: impl Into<String>, label: &str, accent: Color32) {
    let value = value.into();
    egui::Frame::none()
        .fill(super::theme::CARD)
        .stroke(Stroke::new(1.0, accent.linear_multiply(0.45)))
        .rounding(10.0)
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_min_width(150.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).size(22.0).color(accent));
                    ui.label(RichText::new(value).size(22.0).strong());
                });
                ui.add_space(2.0);
                ui.label(RichText::new(label).size(12.5).color(super::theme::TEXT_WEAK));
            });
        });
}

/// 显示秒数的圆形/方块大字（用于倒计时主显示）
pub fn big_timer(ui: &mut Ui, secs_remaining: u64, sub_label: &str) {
    ui.vertical_centered(|ui| {
        let m = secs_remaining / 60;
        let s = secs_remaining % 60;
        ui.label(RichText::new(format!("{m:02}:{s:02}")).size(56.0).monospace().strong());
        ui.label(RichText::new(sub_label).size(14.0).weak());
    });
}
