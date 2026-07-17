// 健康知识库：分类导航 + 默认摘要、按需展开的知识卡片

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::tips::{Tip, TipCategory};
use crate::ui::{theme, widgets};
use crate::ui::widgets::badge;

const NAV_W: f32 = 164.0;

fn rgb(color: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(color.0, color.1, color.2)
}

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    let category_key = egui::Id::new("library_current_category");
    let mut current: TipCategory = ui
        .ctx()
        .memory(|memory| memory.data.get_temp(category_key).unwrap_or(TipCategory::Eyes));

    widgets::page_header(ui, "健康知识", "从护眼到睡眠，按场景找到可立即执行的养护方法。");
    ui.add_space(14.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(NAV_W, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_width(NAV_W);
                for category in TipCategory::all() {
                    if category_button(ui, *category, current == *category) {
                        current = *category;
                    }
                    ui.add_space(3.0);
                }
            },
        );

        ui.add_space(12.0);
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let accent = rgb(current.accent());
                let tips = app.tips.by_category(current);
                widgets::section_header(
                    ui,
                    current.label(),
                    Some(&format!("{} 条知识", tips.len())),
                );
                ui.add_space(8.0);

                let open_key = egui::Id::new(("library_open_tip", current.key()));
                let mut open_title: Option<String> = ui
                    .ctx()
                    .memory(|memory| memory.data.get_temp(open_key).unwrap_or_default());

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if tips.is_empty() {
                            empty_state(ui);
                        }
                        for tip in tips {
                            let expanded = open_title.as_deref() == Some(tip.title.as_str());
                            if tip_card(ui, tip, accent, expanded) {
                                open_title = if expanded {
                                    None
                                } else {
                                    Some(tip.title.clone())
                                };
                                ui.ctx().request_repaint();
                            }
                            ui.add_space(10.0);
                        }
                        ui.add_space(4.0);
                    });

                ui.ctx()
                    .memory_mut(|memory| memory.data.insert_temp(open_key, open_title));
            },
        );
    });

    ui.ctx()
        .memory_mut(|memory| memory.data.insert_temp(category_key, current));
}

fn category_button(ui: &mut egui::Ui, category: TipCategory, selected: bool) -> bool {
    let accent = rgb(category.accent());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 38.0),
        egui::Sense::click(),
    );
    let fill = if selected {
        accent.linear_multiply(0.18)
    } else if response.hovered() {
        theme::CARD_HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 7.0, fill);
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        egui::Align2::LEFT_CENTER,
        format!("{}  {}", category.icon(), category.label()),
        egui::FontId::proportional(13.5),
        if selected { accent } else { theme::TEXT },
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

/// 返回整张卡片是否被点击。
fn tip_card(ui: &mut egui::Ui, tip: &Tip, accent: Color32, expanded: bool) -> bool {
    let response = egui::Frame::none()
        .fill(if expanded { theme::CARD_ALT } else { theme::CARD })
        .stroke(egui::Stroke::new(
            1.0,
            if expanded {
                accent.linear_multiply(0.55)
            } else {
                theme::STROKE
            },
        ))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 40.0).max(180.0));
                    ui.add(
                        egui::Label::new(
                            RichText::new(&tip.title)
                                .size(16.0)
                                .strong()
                                .color(accent),
                        )
                        .wrap(),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if tip.duration_secs > 0 {
                            badge(ui, fmt_duration(tip.duration_secs), accent);
                        }
                        if !tip.frequency.is_empty() {
                            badge(ui, &tip.frequency, theme::TEXT_WEAK);
                        }
                        if tip.office_break {
                            badge(ui, "办公室可做", theme::ACCENT);
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let symbol = if expanded { "⌃" } else { "⌄" };
                    ui.label(RichText::new(symbol).color(theme::TEXT_WEAK));
                });
            });

            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    RichText::new(&tip.benefit)
                        .size(13.5)
                        .color(theme::TEXT),
                )
                .wrap(),
            );

            if expanded {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);
                widgets::section_header(ui, "练习步骤", None);
                ui.add_space(6.0);
                for (index, step) in tip.steps.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.label(
                            RichText::new(format!("{:02}", index + 1))
                                .monospace()
                                .strong()
                                .color(accent),
                        );
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(step).size(13.5).color(theme::TEXT),
                            )
                            .wrap(),
                        );
                    });
                    ui.add_space(4.0);
                }

                if !tip.science.is_empty() {
                    detail_section(ui, "科学依据", &tip.science, theme::INFO);
                }
                if !tip.caution.is_empty() {
                    detail_section(ui, "注意事项", &tip.caution, theme::WARN);
                }
                if !tip.variants.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new("进阶变式").size(13.5).strong().color(theme::PURPLE));
                    for variant in &tip.variants {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new("·").color(theme::PURPLE));
                            ui.add(
                                egui::Label::new(
                                    RichText::new(variant).size(13.5).color(theme::TEXT_WEAK),
                                )
                                .wrap(),
                            );
                        });
                    }
                }
            }
        })
        .response
        .interact(egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

fn detail_section(ui: &mut egui::Ui, title: &str, body: &str, color: Color32) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).size(13.5).strong().color(color));
    ui.add(
        egui::Label::new(RichText::new(body).size(13.5).color(theme::TEXT_WEAK)).wrap(),
    );
}

fn empty_state(ui: &mut egui::Ui) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("此分类暂无内容").color(theme::TEXT_WEAK));
    });
}

fn fmt_duration(seconds: u32) -> String {
    if seconds >= 60 {
        let minutes = seconds / 60;
        let rest = seconds % 60;
        if rest == 0 {
            format!("{minutes} 分钟")
        } else {
            format!("{minutes} 分 {rest} 秒")
        }
    } else {
        format!("{seconds} 秒")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tips::Library;

    fn render_category_height(category: TipCategory, expanded: bool) -> (usize, f32) {
        let library = Library::load().expect("加载 tips.toml");
        let tips = library.by_category(category);
        let count = tips.len();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        let mut content_height = 0.0;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(560.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        for (index, tip) in tips.iter().enumerate() {
                            tip_card(ui, tip, theme::INFO, expanded && index == 0);
                            ui.add_space(10.0);
                        }
                        content_height = ui.min_rect().height();
                    },
                );
            });
        });
        (count, content_height)
    }

    #[test]
    fn all_categories_have_visible_summary_cards() {
        for category in TipCategory::all() {
            let (count, height) = render_category_height(*category, false);
            assert!(count > 0, "分类 {category:?} 没有知识内容");
            assert!(height > count as f32 * 40.0, "摘要卡片疑似渲染塌陷");
        }
    }

    #[test]
    fn expanded_card_is_taller_than_summary_card() {
        let (_, summary_height) = render_category_height(TipCategory::Eyes, false);
        let (_, expanded_height) = render_category_height(TipCategory::Eyes, true);
        assert!(expanded_height > summary_height);
    }

    #[test]
    fn clicking_card_body_toggles_details() {
        let library = Library::load().expect("加载 tips.toml");
        let tip = library
            .by_category(TipCategory::Eyes)
            .into_iter()
            .next()
            .expect("护眼分类应有知识内容");
        let ctx = egui::Context::default();
        // 标题文字区域，不是箭头或卡片空白处。
        let position = egui::pos2(120.0, 30.0);

        let render = |events| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 400.0),
                )),
                events,
                ..Default::default()
            };
            let mut clicked = false;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    clicked = tip_card(ui, tip, theme::INFO, false);
                });
            });
            clicked
        };

        assert!(!render(Vec::new()));
        assert!(!render(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]));
        assert!(render(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]));
    }
}
