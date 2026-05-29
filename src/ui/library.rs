// 健康知识库面板
// 左侧分类导航（带主题色），右侧滚动的富内容卡片：
// 步骤 / 益处 / 频率·时长徽章 / 可折叠的「科学依据·注意事项·进阶变式」

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::tips::{Tip, TipCategory};
use crate::ui::theme;
use crate::ui::widgets::badge;

const NAV_W: f32 = 196.0;

fn rgb(c: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    // 用 ctx.memory 保存当前选中分类，避免 App 结构里再加字段
    let key = egui::Id::new("library_current_category");
    let mut cur: TipCategory = ui
        .ctx()
        .memory(|m| m.data.get_temp(key).unwrap_or(TipCategory::Eyes));

    ui.add_space(2.0);
    ui.heading("📚 健康知识库");
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(8.0);

    ui.horizontal_top(|ui| {
        // ── 左侧分类导航（固定宽度）──
        ui.allocate_ui_with_layout(
            egui::vec2(NAV_W, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_width(NAV_W);
                for c in TipCategory::all() {
                    if category_button(ui, *c, cur == *c) {
                        cur = *c;
                    }
                    ui.add_space(3.0);
                }
            },
        );

        ui.add_space(12.0);

        // ── 右侧内容（显式占满剩余宽高，滚动区才有确定尺寸）──
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let accent = rgb(cur.accent());
                let tips = app.tips.by_category(cur);

                // 当前分类标题 + 条数
                ui.horizontal(|ui| {
                    ui.label(RichText::new(cur.icon()).size(18.0).color(accent));
                    ui.add_space(2.0);
                    ui.label(RichText::new(cur.label()).size(17.0).strong().color(accent));
                    ui.label(
                        RichText::new(format!("· {} 条", tips.len()))
                            .size(13.0)
                            .color(theme::TEXT_WEAK),
                    );
                });
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if tips.is_empty() {
                            ui.label(
                                RichText::new("该分类暂无内容")
                                    .italics()
                                    .color(theme::TEXT_WEAK),
                            );
                        }
                        for tip in tips {
                            tip_card(ui, tip, accent);
                            ui.add_space(12.0);
                        }
                        // 底部留白，最后一张卡片不贴边
                        ui.add_space(4.0);
                    });
            },
        );
    });

    ui.ctx().memory_mut(|m| m.data.insert_temp(key, cur));
}

/// 左侧一个分类按钮，返回是否被点击。选中态用分类主题色高亮。
fn category_button(ui: &mut egui::Ui, c: TipCategory, selected: bool) -> bool {
    let accent = rgb(c.accent());
    let fill = if selected {
        accent.linear_multiply(0.20)
    } else {
        Color32::TRANSPARENT
    };
    let resp = egui::Frame::none()
        .fill(fill)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.set_min_width(NAV_W - 16.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(c.icon())
                        .size(16.0)
                        .color(if selected { accent } else { theme::TEXT_WEAK }),
                );
                ui.add_space(4.0);
                let txt = RichText::new(c.label()).size(14.5);
                ui.label(if selected {
                    txt.strong().color(accent)
                } else {
                    txt.color(theme::TEXT)
                });
            });
        })
        .response;
    let resp = resp.interact(egui::Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// 单张知识卡片
fn tip_card(ui: &mut egui::Ui, tip: &Tip, accent: Color32) {
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, theme::STROKE))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            // 整行内容统一左对齐
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                // 标题（整行，自动换行，绝不截断）
                ui.add(
                    egui::Label::new(RichText::new(&tip.title).size(17.0).strong().color(accent))
                        .wrap(),
                );

                // 频率 / 时长徽章（自动折行）
                if tip.duration_secs > 0 || !tip.frequency.is_empty() {
                    ui.add_space(7.0);
                    ui.horizontal_wrapped(|ui| {
                        if tip.duration_secs > 0 {
                            badge(ui, fmt_duration(tip.duration_secs), accent);
                        }
                        if !tip.frequency.is_empty() {
                            badge(ui, format!("🔁 {}", tip.frequency), theme::TEXT_WEAK);
                        }
                    });
                }

                ui.add_space(11.0);

                // 步骤（序号 + 自动换行的文字）
                for (i, step) in tip.steps.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.label(
                            RichText::new(format!("{}", i + 1))
                                .monospace()
                                .strong()
                                .color(accent),
                        );
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(RichText::new(step).size(14.5).color(theme::TEXT))
                                .wrap(),
                        );
                    });
                    ui.add_space(4.0);
                }

                ui.add_space(8.0);

                // 益处
                ui.horizontal_top(|ui| {
                    ui.label(RichText::new("💡").size(15.0));
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(&tip.benefit)
                                .size(14.0)
                                .color(theme::ACCENT)
                                .italics(),
                        )
                        .wrap(),
                    );
                });

                // 可折叠的进阶信息（按需出现）
                let has_more =
                    !tip.science.is_empty() || !tip.caution.is_empty() || !tip.variants.is_empty();
                if has_more {
                    ui.add_space(8.0);
                    collapsing_block(ui, tip);
                }
            });
        });
}

/// 科学依据 / 注意事项 / 进阶变式 —— 折叠区
fn collapsing_block(ui: &mut egui::Ui, tip: &Tip) {
    let id = ui.make_persistent_id(("tip_more", tip.title.as_str()));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label(
                RichText::new("展开详情（原理 · 注意 · 进阶）")
                    .size(13.0)
                    .color(theme::TEXT_WEAK),
            );
        })
        .body(|ui| {
            ui.add_space(4.0);
            if !tip.science.is_empty() {
                detail_section(ui, "🔬 科学依据", &tip.science, theme::INFO);
            }
            if !tip.caution.is_empty() {
                detail_section(ui, "⚠ 注意事项", &tip.caution, theme::WARN);
            }
            if !tip.variants.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("⭐ 进阶变式").size(13.5).strong().color(theme::PURPLE));
                for v in &tip.variants {
                    ui.horizontal_top(|ui| {
                        ui.label(RichText::new("·").color(theme::PURPLE));
                        ui.add_space(4.0);
                        ui.add(
                            egui::Label::new(RichText::new(v).size(13.5).color(theme::TEXT)).wrap(),
                        );
                    });
                }
            }
        });
}

fn detail_section(ui: &mut egui::Ui, title: &str, body: &str, color: Color32) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).size(13.5).strong().color(color));
    ui.add(egui::Label::new(RichText::new(body).size(13.5).color(theme::TEXT_WEAK)).wrap());
}

fn fmt_duration(secs: u32) -> String {
    if secs >= 60 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("⏱ {m} 分钟")
        } else {
            format!("⏱ {m} 分 {s} 秒")
        }
    } else {
        format!("⏱ {secs} 秒")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tips::Library;

    /// 无头渲染：确认知识库卡片真能产出内容、不会塌陷为空白。
    fn render_category_height(cat: TipCategory) -> (usize, f32) {
        let lib = Library::load().expect("加载 tips.toml");
        let tips_n = lib.by_category(cat).len();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        let mut content_h = 0.0_f32;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let accent = Color32::from_rgb(120, 180, 220);
                ui.allocate_ui_with_layout(
                    egui::vec2(560.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        for tip in lib.by_category(cat) {
                            tip_card(ui, tip, accent);
                            ui.add_space(12.0);
                        }
                        content_h = ui.min_rect().height();
                    },
                );
            });
        });
        (tips_n, content_h)
    }

    #[test]
    fn library_cards_render_with_content() {
        let (n, h) = render_category_height(TipCategory::Eyes);
        assert!(n > 0, "护眼分类应当有 tips");
        // 每张卡至少几十像素高，多张叠加应当远超此阈值；若塌陷为空白会很小
        assert!(h > (n as f32) * 40.0, "卡片疑似塌陷为空白: {n} 张共 {h}px");
    }

    #[test]
    fn all_categories_non_empty_and_render() {
        for c in TipCategory::all() {
            let (n, h) = render_category_height(*c);
            assert!(n > 0, "分类 {:?} 没有 tips", c);
            assert!(h > 40.0, "分类 {:?} 渲染塌陷: {h}px", c);
        }
    }
}
