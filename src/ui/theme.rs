// 全局视觉风格：柔和深色主题 + 统一间距/圆角/字号层级
//
// 在 fonts 之后、App 创建之前由 main.rs 调用一次。

use eframe::egui::{
    self, Color32, FontFamily, FontId, Margin, Rounding, Stroke, TextStyle,
};

// ── 调色板（整个 UI 统一引用，避免散落的魔法色值）─────────────────
pub const BG: Color32 = Color32::from_rgb(0x15, 0x18, 0x1d); // 最底层背景
pub const PANEL: Color32 = Color32::from_rgb(0x1b, 0x1f, 0x26); // 顶栏/侧栏
pub const CARD: Color32 = Color32::from_rgb(0x23, 0x28, 0x31); // 卡片/分组
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x2b, 0x31, 0x3c);
pub const STROKE: Color32 = Color32::from_rgb(0x33, 0x3a, 0x45); // 描边
pub const TEXT: Color32 = Color32::from_rgb(0xe7, 0xea, 0xee); // 主文字
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x97, 0x9f, 0xab); // 次要文字

pub const ACCENT: Color32 = Color32::from_rgb(0x4c, 0xc2, 0x7d); // 品牌绿
pub const INFO: Color32 = Color32::from_rgb(0x6f, 0xb0, 0xe0); // 科学依据·蓝
pub const WARN: Color32 = Color32::from_rgb(0xe0, 0xb1, 0x5e); // 注意事项·琥珀
pub const PURPLE: Color32 = Color32::from_rgb(0xb6, 0x8f, 0xe0); // 进阶变式·紫

/// 安装主题（仅调用一次）。
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // —— 字号层级 ——
    use FontFamily::{Monospace, Proportional};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(21.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (TextStyle::Small, FontId::new(12.5, Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, Monospace)),
    ]
    .into();

    // —— 间距 ——
    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 8.0);
    s.button_padding = egui::vec2(12.0, 6.0);
    s.menu_margin = Margin::same(8.0);
    s.window_margin = Margin::same(12.0);
    s.indent = 18.0;
    s.interact_size.y = 30.0;

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

    let rounding = Rounding::same(8.0);
    v.window_rounding = Rounding::same(10.0);
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
