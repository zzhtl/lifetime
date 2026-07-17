// 通用 UI 小部件

use eframe::egui::{self, Align2, Color32, RichText, Stroke, Ui};

use super::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Success,
    Error,
}

/// 小标签（圆角胶囊），用于时长/频率/分类等元信息。
pub fn badge(ui: &mut Ui, text: impl Into<String>, color: Color32) {
    egui::Frame::none()
        .fill(color.linear_multiply(0.18))
        .rounding(egui::Rounding::same(6.0))
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
        .rounding(8.0)
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width().min(150.0));
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

pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(22.0).strong().color(theme::TEXT));
    ui.add_space(2.0);
    ui.add(
        egui::Label::new(RichText::new(subtitle).size(13.0).color(theme::TEXT_WEAK)).wrap(),
    );
}

pub fn section_header(ui: &mut Ui, title: &str, meta: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(15.0).strong().color(theme::TEXT));
        if let Some(meta) = meta {
            ui.label(RichText::new(meta).size(12.5).color(theme::TEXT_WEAK));
        }
    });
}

/// 固定高度的侧栏项，避免图标或未保存标记引发布局跳动。
pub fn nav_item(ui: &mut Ui, icon: &str, label: &str, selected: bool, dirty: bool) -> egui::Response {
    let size = egui::vec2(ui.available_width(), 40.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let fill = if selected {
        theme::ACCENT.linear_multiply(0.18)
    } else if response.hovered() {
        theme::CARD_HOVER
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if selected {
        Stroke::new(1.0, theme::ACCENT.linear_multiply(0.55))
    } else {
        Stroke::NONE
    };
    ui.painter().rect(rect, 7.0, fill, stroke);
    if selected {
        let marker = egui::Rect::from_min_max(
            rect.left_top() + egui::vec2(0.0, 8.0),
            rect.left_bottom() + egui::vec2(3.0, -8.0),
        );
        ui.painter().rect_filled(marker, 2.0, theme::ACCENT);
    }
    let color = if selected { theme::ACCENT } else { theme::TEXT };
    ui.painter().text(
        rect.left_center() + egui::vec2(14.0, 0.0),
        Align2::LEFT_CENTER,
        format!("{icon}   {label}"),
        egui::FontId::proportional(14.5),
        color,
    );
    if dirty {
        ui.painter().circle_filled(rect.right_center() - egui::vec2(12.0, 0.0), 3.5, theme::WARN);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn status_badge(ui: &mut Ui, text: &str, color: Color32) {
    egui::Frame::none()
        .fill(color.linear_multiply(0.14))
        .stroke(Stroke::new(1.0, color.linear_multiply(0.45)))
        .rounding(6.0)
        .inner_margin(egui::Margin::symmetric(9.0, 4.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.5).color(color));
        });
}

pub fn toast(ctx: &egui::Context, kind: NoticeKind, message: &str) -> bool {
    let accent = match kind {
        NoticeKind::Success => theme::ACCENT,
        NoticeKind::Error => theme::DANGER,
    };
    let mut dismiss = false;
    egui::Area::new(egui::Id::new("global-notice"))
        .anchor(Align2::RIGHT_TOP, [-20.0, 68.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::CARD_ALT)
                .stroke(Stroke::new(1.0, accent.linear_multiply(0.7)))
                .rounding(8.0)
                .inner_margin(egui::Margin::symmetric(12.0, 9.0))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if kind == NoticeKind::Success { "✓" } else { "!" }).strong().color(accent));
                        ui.add(egui::Label::new(RichText::new(message).color(theme::TEXT)).wrap());
                        if ui.button("×").on_hover_text("关闭").clicked() {
                            dismiss = true;
                        }
                    });
                });
        });
    dismiss
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
