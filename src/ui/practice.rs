// 养生修炼：分类、境界反馈、摘要卡片与每日打卡

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::practices::{realm_progress, Practice, PracticeCategory, RealmProgress};
use crate::ui::{theme, widgets};
use crate::ui::widgets::badge;

const NAV_W: f32 = 174.0;

fn rgb(color: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(color.0, color.1, color.2)
}

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    let category_key = egui::Id::new("practice_current_category");
    let mut current: PracticeCategory = ui
        .ctx()
        .memory(|memory| memory.data.get_temp(category_key).unwrap_or(PracticeCategory::Diet));
    if current == PracticeCategory::Breathing {
        current = PracticeCategory::Diet;
    }

    let realm = realm_progress(app.cultivation.points);
    let streak = crate::db::practice_streak(&app.db).unwrap_or(0);
    let mut pending_checkin: Option<(String, String)> = None;

    widgets::page_header(ui, "养生修炼", "以内经为纲，节饮食、常起居、调形神；每日一修，重在可持续。");
    ui.add_space(14.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(NAV_W, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_width(NAV_W);
                for category in PracticeCategory::all() {
                    if *category == PracticeCategory::Breathing {
                        continue;
                    }
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
                let practices = app.practices.by_category(current);
                let logged: std::collections::HashSet<String> =
                    app.cultivation.today_logged.iter().cloned().collect();

                widgets::section_header(
                    ui,
                    current.label(),
                    Some(&format!("{} 项功法", practices.len())),
                );
                ui.add_space(8.0);
                realm_banner(ui, &realm, streak, accent);
                ui.add_space(10.0);

                let open_key = egui::Id::new(("practice_open_item", current.key()));
                let mut open_title: Option<String> = ui
                    .ctx()
                    .memory(|memory| memory.data.get_temp(open_key).unwrap_or_default());

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if practices.is_empty() {
                            ui.label(RichText::new("该分类暂无内容").color(theme::TEXT_WEAK));
                        }
                        for practice in practices {
                            let expanded = open_title.as_deref() == Some(practice.title.as_str());
                            if practice_card(ui, practice, accent, expanded) {
                                open_title = if expanded {
                                    None
                                } else {
                                    Some(practice.title.clone())
                                };
                                ui.ctx().request_repaint();
                            }
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                let done = logged.contains(&practice.title);
                                if checkin_button(ui, done, accent) {
                                    pending_checkin = Some((
                                        practice.category.key().to_string(),
                                        practice.title.clone(),
                                    ));
                                }
                            });
                            ui.add_space(12.0);
                        }
                        ui.add_space(4.0);
                    });

                ui.ctx()
                    .memory_mut(|memory| memory.data.insert_temp(open_key, open_title));
            },
        );
    });

    if let Some((category, title)) = pending_checkin {
        app.log_practice(&category, title);
    }
    ui.ctx()
        .memory_mut(|memory| memory.data.insert_temp(category_key, current));
}

fn category_button(ui: &mut egui::Ui, category: PracticeCategory, selected: bool) -> bool {
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

fn realm_banner(ui: &mut egui::Ui, realm: &RealmProgress, streak: i64, accent: Color32) {
    egui::Frame::none()
        .fill(theme::CARD_ALT)
        .stroke(egui::Stroke::new(1.0, accent.linear_multiply(0.45)))
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(realm.name).size(15.0).strong().color(accent));
                ui.label(
                    RichText::new(format!("修为 {}", realm.points))
                        .size(12.0)
                        .color(theme::TEXT_WEAK),
                );
                if streak > 0 {
                    ui.label(
                        RichText::new(format!("连续 {} 天", streak))
                            .size(12.0)
                            .color(theme::WARN),
                    );
                }
            });
            ui.add_space(5.0);
            let hint = match realm.next_name {
                Some(next) => format!("距「{next}」还需 {} 次", realm.need),
                None => "已臻化境".to_string(),
            };
            ui.add(
                egui::ProgressBar::new(realm.ratio)
                    .fill(accent)
                    .text(RichText::new(hint).size(11.0)),
            );
        });
}

/// 返回整张卡片是否被点击。
pub(crate) fn practice_card(
    ui: &mut egui::Ui,
    practice: &Practice,
    accent: Color32,
    expanded: bool,
) -> bool {
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
                            RichText::new(&practice.title)
                                .size(16.0)
                                .strong()
                                .color(accent),
                        )
                        .wrap(),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        badge(ui, practice.stage.label(), accent);
                        if practice.duration_mins > 0 {
                            badge(ui, format!("{} 分钟", practice.duration_mins), theme::TEXT_WEAK);
                        }
                        if !practice.frequency.is_empty() {
                            badge(ui, &practice.frequency, theme::TEXT_WEAK);
                        }
                        for scene in &practice.scenes {
                            badge(ui, scene.label(), theme::PURPLE);
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    ui.label(
                        RichText::new(if expanded { "⌃" } else { "⌄" })
                            .color(theme::TEXT_WEAK),
                    );
                });
            });

            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    RichText::new(&practice.summary)
                        .size(13.5)
                        .color(theme::TEXT),
                )
                .wrap(),
            );
            ui.add_space(5.0);
            ui.add(
                egui::Label::new(
                    RichText::new(&practice.benefit)
                        .size(13.0)
                        .color(theme::ACCENT),
                )
                .wrap(),
            );

            if expanded {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);
                widgets::section_header(ui, "修炼步骤", None);
                ui.add_space(6.0);
                for (index, step) in practice.steps.iter().enumerate() {
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
                detail_line(ui, "今解", &practice.science, theme::INFO);
                detail_line(ui, "戒偏", &practice.caution, theme::WARN);

                if !practice.sources.is_empty() {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("原典与依据")
                            .size(13.5)
                            .strong()
                            .color(theme::TEXT_WEAK),
                    );
                    for source in &practice.sources {
                        ui.horizontal_wrapped(|ui| {
                            badge(ui, source.level.label(), theme::TEXT_WEAK);
                            ui.hyperlink_to(&source.name, &source.url);
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

pub(crate) fn checkin_button(ui: &mut egui::Ui, done: bool, accent: Color32) -> bool {
    if done {
        ui.add_enabled(false, egui::Button::new("✓ 今日已修"));
        false
    } else {
        ui.add(
            egui::Button::new(RichText::new("✓ 今日修炼").strong().color(accent))
                .min_size(egui::vec2(108.0, 30.0)),
        )
        .clicked()
    }
}

fn detail_line(ui: &mut egui::Ui, title: &str, body: &str, color: Color32) {
    if body.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(RichText::new(title).size(13.5).strong().color(color));
    ui.add(
        egui::Label::new(RichText::new(body).size(13.5).color(theme::TEXT_WEAK)).wrap(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::practices::PracticeLibrary;

    fn render_category_height(category: PracticeCategory, expanded: bool) -> (usize, f32) {
        let library = PracticeLibrary::load().expect("加载 practices.toml");
        let practices = library.by_category(category);
        let count = practices.len();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        };
        let mut content_height = 0.0;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(620.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        for (index, practice) in practices.iter().enumerate() {
                            practice_card(ui, practice, theme::ACCENT, expanded && index == 0);
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
    fn all_practice_categories_render_as_summaries() {
        for category in PracticeCategory::all() {
            let (count, height) = render_category_height(*category, false);
            assert!(count > 0, "修炼分类 {category:?} 没有内容");
            assert!(height > count as f32 * 45.0, "摘要卡片疑似渲染塌陷");
        }
    }

    #[test]
    fn expanded_practice_and_realm_states_render() {
        let (_, summary_height) = render_category_height(PracticeCategory::Diet, false);
        let (_, expanded_height) = render_category_height(PracticeCategory::Diet, true);
        assert!(expanded_height > summary_height);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                realm_banner(ui, &realm_progress(10), 3, theme::ACCENT);
                realm_banner(ui, &realm_progress(1000), 0, theme::WARN);
                let _ = checkin_button(ui, false, theme::ACCENT);
                let _ = checkin_button(ui, true, theme::ACCENT);
            });
        });
    }

    #[test]
    fn clicking_practice_card_body_toggles_details() {
        let library = PracticeLibrary::load().expect("加载 practices.toml");
        let practice = library
            .by_category(PracticeCategory::Diet)
            .into_iter()
            .next()
            .expect("饮食分类应有修炼内容");
        let ctx = egui::Context::default();
        // 标题文字区域，不是箭头或卡片空白处。
        let position = egui::pos2(120.0, 30.0);

        let render = |events| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 480.0),
                )),
                events,
                ..Default::default()
            };
            let mut clicked = false;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    clicked = practice_card(ui, practice, theme::ACCENT, false);
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
