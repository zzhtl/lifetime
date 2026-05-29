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
    let remaining = b.remaining;
    let skip_left = b.skip_available_in;
    let title = b.tip_title.clone();
    let steps = b.tip_steps.clone();
    let benefit = b.tip_benefit.clone();

    let viewport_id = egui::ViewportId::from_hash_of("lifetime-break");
    let builder = egui::ViewportBuilder::default()
        .with_title("Lifetime · 该休息了")
        .with_inner_size([700.0, 460.0])
        .with_min_inner_size([520.0, 360.0])
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
            .frame(egui::Frame::none().fill(Color32::from_rgb(20, 24, 40)))
            .show(ctx, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("{} · 该休息了", kind.label()))
                            .size(22.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(8.0);
                    // 大字倒计时
                    let m = remaining / 60;
                    let s = remaining % 60;
                    ui.label(
                        RichText::new(format!("{m:02}:{s:02}"))
                            .size(72.0)
                            .monospace()
                            .strong()
                            .color(Color32::from_rgb(180, 230, 255)),
                    );
                    // 进度条
                    let ratio = (total - remaining) as f32 / total as f32;
                    ui.add(
                        egui::ProgressBar::new(ratio.clamp(0.0, 1.0))
                            .desired_width(420.0)
                            .text(""),
                    );

                    ui.add_space(20.0);

                    egui::Frame::none()
                        .fill(Color32::from_rgb(34, 40, 60))
                        .rounding(10.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.set_width(520.0);
                            // 卡片内统一左对齐，长文本自动换行，避免居中换行难读 / 被截断
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                ui.label(
                                    RichText::new(&title)
                                        .size(18.0)
                                        .strong()
                                        .color(Color32::from_rgb(255, 230, 180)),
                                );
                                ui.add_space(6.0);
                                if steps.is_empty() {
                                    ui.label(
                                        RichText::new(kind.brief()).color(Color32::LIGHT_GRAY),
                                    );
                                } else {
                                    for (i, step) in steps.iter().enumerate() {
                                        ui.horizontal_top(|ui| {
                                            ui.label(
                                                RichText::new(format!("{}.", i + 1))
                                                    .monospace()
                                                    .strong()
                                                    .color(Color32::from_rgb(180, 230, 255)),
                                            );
                                            ui.add_space(2.0);
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(step)
                                                        .color(Color32::from_rgb(220, 230, 240))
                                                        .size(15.0),
                                                )
                                                .wrap(),
                                            );
                                        });
                                    }
                                }
                                if !benefit.is_empty() {
                                    ui.add_space(6.0);
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

                    ui.add_space(20.0);

                    // 跳过 / 完成按钮
                    ui.horizontal(|ui| {
                        ui.add_space(120.0);
                        let skip_label = if skip_left > 0 {
                            format!("⏭ 跳过 (再 {} s)", skip_left)
                        } else {
                            "⏭ 跳过".to_string()
                        };
                        let skip_btn = ui.add_enabled(
                            skip_left == 0,
                            egui::Button::new(RichText::new(skip_label).size(14.0))
                                .min_size(egui::vec2(140.0, 36.0)),
                        );
                        if skip_btn.clicked() {
                            close_requested = true;
                            acknowledged = false;
                        }

                        ui.add_space(20.0);

                        if ui
                            .add(
                                egui::Button::new(RichText::new("✅ 完成").size(14.0).strong())
                                    .min_size(egui::vec2(140.0, 36.0)),
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
        if acknowledged {
            app.send(Command::AcknowledgeBreak(kind));
        } else {
            app.send(Command::Skip(kind));
        }
    }
}
