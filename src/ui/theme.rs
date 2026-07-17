// 全局视觉风格：现代桌面骨架 + 东方养生配色
//
// 在 fonts 之后、App 创建之前由 main.rs 调用一次。

use eframe::egui::{
    self, Color32, FontFamily, FontId, Margin, Rounding, Stroke, TextStyle,
};

// ── 调色板（墨、玉、金、朱、青）──────────────────────────────
pub const BG: Color32 = Color32::from_rgb(0x10, 0x16, 0x14);
pub const PANEL: Color32 = Color32::from_rgb(0x16, 0x1e, 0x1b);
pub const CARD: Color32 = Color32::from_rgb(0x1c, 0x27, 0x22);
pub const CARD_ALT: Color32 = Color32::from_rgb(0x22, 0x2f, 0x29);
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x27, 0x35, 0x2f);
pub const STROKE: Color32 = Color32::from_rgb(0x30, 0x41, 0x39);
pub const TEXT: Color32 = Color32::from_rgb(0xec, 0xf1, 0xee);
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x97, 0xa8, 0x9f);

pub const ACCENT: Color32 = Color32::from_rgb(0x2d, 0xb6, 0x9c);
pub const INFO: Color32 = Color32::from_rgb(0x72, 0xa9, 0xc5);
pub const WARN: Color32 = Color32::from_rgb(0xd4, 0xb3, 0x6a);
pub const DANGER: Color32 = Color32::from_rgb(0xd7, 0x74, 0x61);
pub const PURPLE: Color32 = Color32::from_rgb(0xa5, 0x8b, 0xc6);

/// 安装主题（仅调用一次）。
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // —— 字号层级 ——
    use FontFamily::{Monospace, Proportional};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (TextStyle::Small, FontId::new(12.5, Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, Monospace)),
    ]
    .into();

    // —— 间距 ——
    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 8.0);
    s.button_padding = egui::vec2(12.0, 7.0);
    s.menu_margin = Margin::same(8.0);
    s.window_margin = Margin::same(12.0);
    s.indent = 18.0;
    s.interact_size.y = 32.0;

    // —— 配色 ——
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = PANEL;
    v.window_fill = BG;
    v.faint_bg_color = CARD;
    v.extreme_bg_color = Color32::from_rgb(0x12, 0x15, 0x19);
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = INFO;
    v.selection.bg_fill = ACCENT.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    let rounding = Rounding::same(7.0);
    v.window_rounding = Rounding::same(8.0);
    v.menu_rounding = rounding;

    // 各交互态控件
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, STROKE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_WEAK);
    w.noninteractive.rounding = rounding;

    w.inactive.bg_fill = CARD;
    w.inactive.weak_bg_fill = CARD;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.rounding = rounding;

    w.hovered.bg_fill = CARD_HOVER;
    w.hovered.weak_bg_fill = CARD_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, ACCENT.linear_multiply(0.6));
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.rounding = rounding;

    w.active.bg_fill = ACCENT.linear_multiply(0.45);
    w.active.weak_bg_fill = ACCENT.linear_multiply(0.45);
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0, TEXT);
    w.active.rounding = rounding;

    w.open.bg_fill = CARD;
    w.open.weak_bg_fill = CARD;
    w.open.bg_stroke = Stroke::new(1.0, STROKE);
    w.open.fg_stroke = Stroke::new(1.0, TEXT);
    w.open.rounding = rounding;

    style.visuals = v;
    ctx.set_style(style);
}
