// 养生修炼面板
// 以内经八纲展示长期健康体系：纲领导航 + 修炼卡片 + 来源依据。

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::practices::{realm_progress, Practice, PracticeCategory, RealmProgress};
use crate::ui::theme;
use crate::ui::widgets::badge;

const NAV_W: f32 = 208.0;

fn rgb(c: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    let key = egui::Id::new("practice_current_category");
    let mut cur: PracticeCategory = ui
        .ctx()
        .memory(|m| m.data.get_temp(key).unwrap_or(PracticeCategory::Diet));

    // 修为境界横幅数据：先 Copy/查出来，避免与下方 practices 借用打架
    let realm = realm_progress(app.cultivation.points);
    let streak = crate::db::practice_streak(&app.db).unwrap_or(0);
    // 打卡点击收集：闭包内 practices 正借用 app，不能同时可变借用，退出后再统一写库
    let mut pending: Option<(String, String)> = None;

    ui.add_space(2.0);
    ui.heading("☯ 养生修炼");
    ui.add_space(4.0);
    ui.add(
        egui::Label::new(
            RichText::new("以内经为纲：法阴阳、节饮食、常起居、调形神。")
                .size(13.0)
                .color(theme::TEXT_WEAK),
        )
        .wrap(),
    );
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(8.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(NAV_W, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_width(NAV_W);
                for c in PracticeCategory::all() {
                    if category_button(ui, *c, cur == *c) {
                        cur = *c;
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
                let accent = rgb(cur.accent());
                let logged: std::collections::HashSet<String> =
                    app.cultivation.today_logged.iter().cloned().collect();
                let practices = app.practices.by_category(cur);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(cur.icon()).size(18.0).color(accent));
                    ui.add_space(2.0);
                    ui.label(RichText::new(cur.label()).size(17.0).strong().color(accent));
                    ui.label(
                        RichText::new(format!("· {} 法", practices.len()))
                            .size(13.0)
                            .color(theme::TEXT_WEAK),
                    );
                });
                ui.add_space(8.0);

                // 修为境界横幅：打卡成长的核心反馈
                realm_banner(ui, &realm, streak, accent);
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if practices.is_empty() {
                            ui.label(
                                RichText::new("该纲暂无修炼内容")
                                    .italics()
                                    .color(theme::TEXT_WEAK),
                            );
                        }
                        for practice in practices {
                            practice_card(ui, practice, accent);
                            // 打卡按钮：今日已修则禁用
                            let done = logged.contains(&practice.title);
                            if checkin_button(ui, done, accent) {
                                pending = Some((
                                    practice.category.key().to_string(),
                                    practice.title.clone(),
                                ));
                            }
                            ui.add_space(12.0);
                        }
                        ui.add_space(4.0);
                    });
            },
        );
    });

    // practices 借用已随闭包结束，安全地写入打卡
    if let Some((category, title)) = pending {
        app.log_practice(&category, title);
    }

    ui.ctx().memory_mut(|m| m.data.insert_temp(key, cur));
}

fn category_button(ui: &mut egui::Ui, c: PracticeCategory, selected: bool) -> bool {
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

fn realm_banner(ui: &mut egui::Ui, realm: &RealmProgress, streak: i64, accent: Color32) {
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, accent.linear_multiply(0.6)))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(12.0, 6.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("⛰ {}", realm.name))
                        .size(15.0)
                        .strong()
                        .color(accent),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("修为 {}", realm.points))
                        .size(12.0)
                        .color(theme::TEXT_WEAK),
                );
                if streak > 0 {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("🔥 连续 {} 天", streak))
                            .size(12.0)
                            .color(theme::WARN),
                    );
                }
            });
            ui.add_space(4.0);
            let hint = match realm.next_name {
                Some(next) => format!("距「{}」还需 {} 次", next, realm.need),
                None => "已臻化境".to_string(),
            };
            ui.add(egui::ProgressBar::new(realm.ratio).text(RichText::new(hint).size(11.0)));
        });
}

fn checkin_button(ui: &mut egui::Ui, done: bool, accent: Color32) -> bool {
    if done {
        ui.add_enabled(
            false,
            egui::Button::new(RichText::new("✅ 今日已修").size(13.0)),
        );
        false
    } else {
        ui.add(egui::Button::new(
            RichText::new("✓ 今日修炼打卡")
                .size(13.0)
                .strong()
                .color(accent),
        ))
        .clicked()
    }
}

fn practice_card(ui: &mut egui::Ui, practice: &Practice, accent: Color32) {
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, theme::STROKE))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(&practice.title)
                            .size(17.0)
                            .strong()
                            .color(accent),
                    )
                    .wrap(),
                );
                ui.add_space(7.0);

                ui.horizontal_wrapped(|ui| {
                    badge(ui, practice.stage.label(), accent);
                    if practice.duration_mins > 0 {
                        badge(ui, format!("⏱ {} 分钟", practice.duration_mins), theme::TEXT_WEAK);
                    }
                    if !practice.frequency.is_empty() {
                        badge(ui, format!("🔁 {}", practice.frequency), theme::TEXT_WEAK);
                    }
                    for scene in &practice.scenes {
                        badge(ui, scene.label(), theme::PURPLE);
                    }
                });

                ui.add_space(10.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(&practice.summary)
                            .size(14.5)
                            .color(theme::TEXT)
                            .italics(),
                    )
                    .wrap(),
                );
                ui.add_space(10.0);

                for (i, step) in practice.steps.iter().enumerate() {
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
                detail_line(ui, "得益", &practice.benefit, theme::ACCENT);
                detail_line(ui, "今解", &practice.science, theme::INFO);
                detail_line(ui, "戒偏", &practice.caution, theme::WARN);
                ui.add_space(6.0);
                sources_block(ui, practice);
            });
        });
}

fn detail_line(ui: &mut egui::Ui, title: &str, body: &str, color: Color32) {
    ui.horizontal_top(|ui| {
        ui.label(RichText::new(title).size(13.5).strong().color(color));
        ui.add_space(4.0);
        ui.add(egui::Label::new(RichText::new(body).size(13.5).color(theme::TEXT_WEAK)).wrap());
    });
}

fn sources_block(ui: &mut egui::Ui, practice: &Practice) {
    let id = ui.make_persistent_id(("practice_sources", practice.title.as_str()));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label(
                RichText::new("展开来源（原典 · 依据）")
                    .size(13.0)
                    .color(theme::TEXT_WEAK),
            );
        })
        .body(|ui| {
            ui.add_space(4.0);
            for source in &practice.sources {
                ui.horizontal_wrapped(|ui| {
                    badge(ui, source.level.label(), theme::TEXT_WEAK);
                    ui.hyperlink_to(&source.name, &source.url);
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::practices::PracticeLibrary;

    fn render_category_height(cat: PracticeCategory) -> (usize, f32) {
        let lib = PracticeLibrary::load().expect("加载 practices.toml");
        let n = lib.by_category(cat).len();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        };
        let mut content_h = 0.0_f32;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let accent = Color32::from_rgb(150, 190, 120);
                ui.allocate_ui_with_layout(
                    egui::vec2(620.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        for practice in lib.by_category(cat) {
                            practice_card(ui, practice, accent);
                            ui.add_space(12.0);
                        }
                        content_h = ui.min_rect().height();
                    },
                );
            });
        });
        (n, content_h)
    }

    #[test]
    fn all_practice_categories_render() {
        for c in PracticeCategory::all() {
            let (n, h) = render_category_height(*c);
            assert!(n > 0, "修炼分类 {:?} 没有内容", c);
            assert!(h > 60.0, "修炼分类 {:?} 渲染塌陷: {h}px", c);
        }
    }

    #[test]
    fn realm_banner_and_checkin_render_without_panic() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let accent = Color32::from_rgb(200, 180, 100);
                // 有下一境界 + 连续天数 > 0
                realm_banner(ui, &crate::practices::realm_progress(10), 3, accent);
                // 达顶境界（next_name = None 分支）+ 连续天数 = 0
                realm_banner(ui, &crate::practices::realm_progress(1000), 0, accent);
                // 打卡按钮两态
                let _ = checkin_button(ui, false, accent);
                let _ = checkin_button(ui, true, accent);
            });
        });
    }
}
