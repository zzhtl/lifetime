// 强制休息模态窗
// 用 eframe::egui::ViewportBuilder 起一个独立 OS 窗口，置顶 + 最大化
// 倒计时归零 / 跳过冷却结束后用户点跳过 → 关窗
//
// 注意：跳过按钮带 15 秒（可配）冷却防误触

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::scheduler::Command;

pub fn render_break_viewport(app: &mut App, ctx: &egui::Context) {
    let Some(b) = app.pending_break.as_ref() else {
        return;
    };
    let kind = b.kind;
    let total = b.total_secs.max(1);
    let remaining = b.remaining; // 整体剩余（进度条用）
    let skip_left = b.skip_available_in;
    // 当前小节
    let seg_remaining = b.seg_remaining;
    let seg_index = b.seg_index;
    let seg_count = b.segments.len();
    let cur = b.segments.get(seg_index);
    let title = cur
        .map(|s| s.title.clone())
        .unwrap_or_else(|| kind.label().to_string());
    let steps = cur.map(|s| s.steps.clone()).unwrap_or_default();
    let benefit = cur.map(|s| s.benefit.clone()).unwrap_or_default();
    let cat_label = cur.map(|s| s.category.label()).unwrap_or("");
    let next_title = b.segments.get(seg_index + 1).map(|s| s.title.clone());

    let viewport_id = egui::ViewportId::from_hash_of("lifetime-break");
    let builder = egui::ViewportBuilder::default()
        .with_title("Lifetime · 该休息了")
        .with_inner_size([760.0, 620.0])
        .with_min_inner_size([620.0, 500.0])
        .with_always_on_top()
        .with_decorations(true)
        .with_maximized(false);

    let mut close_requested = false;
    let mut acknowledged = false;

    ctx.show_viewport_immediate(viewport_id, builder, |ctx, _vc| {
        if ctx.input(|i| i.viewport().close_requested()) {
            close_requested = true;
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 24, 40))
                    .inner_margin(24.0),
            )
            .show(ctx, |ui| {
                let content_width = ui.available_width().clamp(480.0, 640.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("{} · 该休息了", kind.label()))
                            .size(22.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(4.0);
                    // 跟练进度：第 x/n 节 · 部位 · 整体剩余
                    let overall_m = remaining / 60;
                    let overall_s = remaining % 60;
                    ui.label(
                        RichText::new(format!(
                            "第 {}/{} 节 · {} · 整体剩余 {:02}:{:02}",
                            seg_index + 1,
                            seg_count.max(1),
                            cat_label,
                            overall_m,
                            overall_s,
                        ))
                        .size(14.0)
                        .color(Color32::from_rgb(160, 200, 230)),
                    );
                    ui.add_space(6.0);
                    // 大字倒计时（当前小节）
                    let m = seg_remaining / 60;
                    let s = seg_remaining % 60;
                    ui.label(
                        RichText::new(format!("{m:02}:{s:02}"))
                            .size(68.0)
                            .monospace()
                            .strong()
                            .color(Color32::from_rgb(180, 230, 255)),
                    );
                    // 整体进度条
                    let ratio = (total - remaining) as f32 / total as f32;
                    ui.add(
                        egui::ProgressBar::new(ratio.clamp(0.0, 1.0))
                            .desired_width(content_width.min(460.0))
                            .text(""),
                    );

                    ui.add_space(18.0);

                    let card_height = (ui.available_height() - 104.0).max(220.0);

                    egui::Frame::none()
                        .fill(Color32::from_rgb(34, 40, 60))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(54, 64, 88)))
                        .rounding(10.0)
                        .inner_margin(18.0)
                        .show(ui, |ui| {
                            ui.set_width(content_width);
                            ui.set_min_height(card_height);

                            egui::ScrollArea::vertical()
                                .max_height(card_height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    // 卡片内统一左对齐；内容超高时滚动，避免底部文字被裁掉
                                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&title)
                                                    .size(18.0)
                                                    .strong()
                                                    .color(Color32::from_rgb(255, 230, 180)),
                                            )
                                            .wrap(),
                                        );
                                        ui.add_space(8.0);
                                        if steps.is_empty() {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(kind.brief())
                                                        .color(Color32::LIGHT_GRAY),
                                                )
                                                .wrap(),
                                            );
                                        } else {
                                            for (i, step) in steps.iter().enumerate() {
                                                ui.horizontal_top(|ui| {
                                                    ui.set_width(ui.available_width());
                                                    ui.label(
                                                        RichText::new(format!("{}.", i + 1))
                                                            .monospace()
                                                            .strong()
                                                            .color(Color32::from_rgb(180, 230, 255)),
                                                    );
                                                    ui.add_space(4.0);
                                                    let text_width = ui.available_width().max(120.0);
                                                    ui.allocate_ui_with_layout(
                                                        egui::vec2(text_width, 0.0),
                                                        egui::Layout::top_down(egui::Align::Min),
                                                        |ui| {
                                                            ui.set_width(text_width);
                                                            ui.add(
                                                                egui::Label::new(
                                                                    RichText::new(step)
                                                                        .color(Color32::from_rgb(
                                                                            220, 230, 240,
                                                                        ))
                                                                        .size(15.0),
                                                                )
                                                                .wrap(),
                                                            );
                                                        },
                                                    );
                                                });
                                                ui.add_space(4.0);
                                            }
                                        }
                                        if !benefit.is_empty() {
                                            ui.add_space(8.0);
                                            ui.separator();
                                            ui.add_space(8.0);
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(format!("💡 {benefit}"))
                                                        .italics()
                                                        .color(Color32::from_rgb(150, 220, 150)),
                                                )
                                                .wrap(),
                                            );
                                        }
                                    });
                                });
                        });

                    ui.add_space(10.0);
                    // 下一节预告，帮助跟练有节奏地衔接
                    let next_hint = match &next_title {
                        Some(t) => format!("下一节：{t}"),
                        None => "最后一节，做完就完成啦".to_string(),
                    };
                    ui.label(RichText::new(next_hint).size(13.0).color(Color32::from_rgb(150, 170, 190)));

                    ui.add_space(12.0);

                    // 跳过 / 完成按钮
                    ui.horizontal(|ui| {
                        let button_width = 140.0;
                        let button_gap = 20.0;
                        let buttons_width = button_width * 2.0 + button_gap;
                        ui.add_space(((ui.available_width() - buttons_width) / 2.0).max(0.0));
                        let skip_label = if skip_left > 0 {
                            format!("⏭ 跳过 (再 {} s)", skip_left)
                        } else {
                            "⏭ 跳过".to_string()
                        };
                        let skip_btn = ui.add_enabled(
                            skip_left == 0,
                            egui::Button::new(RichText::new(skip_label).size(14.0))
                                .min_size(egui::vec2(button_width, 36.0)),
                        );
                        if skip_btn.clicked() {
                            close_requested = true;
                            acknowledged = false;
                        }

                        ui.add_space(20.0);

                        if ui
                            .add(
                                egui::Button::new(RichText::new("✅ 完成").size(14.0).strong())
                                    .min_size(egui::vec2(button_width, 36.0)),
                            )
                            .clicked()
                        {
                            acknowledged = true;
                            close_requested = true;
                        }
                    });
                });
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    });

    if close_requested {
        app.pending_break = None;
        // 记录跟练结果（完成 / 跳过），用于"今日跟练完成度"
        app.record_big_break(acknowledged);
        if acknowledged {
            app.send(Command::AcknowledgeBreak(kind));
        } else {
            app.send(Command::Skip(kind));
        }
    }
}
