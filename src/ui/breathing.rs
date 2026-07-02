// 呼吸法门（顶层页）
// 上：可视化呼吸引导器（动画节拍器 + 循证预设 + 计时计数）
// 下：呼吸法门图文库（传统 + 现代循证法门，可「跟练」联动上方练习台）

use std::collections::HashSet;

use eframe::egui::{self, Color32, RichText};

use crate::app::App;
use crate::breathing::{PhaseAt, PhaseKind, BREATHING_PATTERNS};
use crate::practices::PracticeCategory;
use crate::reminders::Intensity;
use crate::scheduler::Command;
use crate::ui::practice::{checkin_button, practice_card};
use crate::ui::theme;
use crate::ui::widgets::{badge, stat_card};

fn rgb(c: (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.heading("🌬 呼吸法门");
    ui.add_space(4.0);
    ui.add(
        egui::Label::new(
            RichText::new("选一种呼吸法，点开始，让圆圈带你一呼一吸。吸气时圆圈涨大，呼气时缩小。")
                .size(13.0)
                .color(theme::TEXT_WEAK),
        )
        .wrap(),
    );
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            pacer_section(app, ui);
            ui.add_space(14.0);
            stats_banner(app, ui);
            ui.add_space(12.0);
            library_section(app, ui);
        });
}

/// 呼吸练习台：预设选择 → 完成检测 → 切拍提示音 → 动画圆圈 → 进度 → 控制条
fn pacer_section(app: &mut App, ui: &mut egui::Ui) {
    // ① 预设选择器（点击后统一应用，避免遍历时可变借用冲突）
    let cur_idx = app.breathing.pattern_idx;
    let mut select: Option<usize> = None;
    ui.horizontal_wrapped(|ui| {
        for (i, pat) in BREATHING_PATTERNS.iter().enumerate() {
            let selected = i == cur_idx;
            let accent = rgb(pat.accent);
            let txt = RichText::new(pat.name)
                .size(13.5)
                .color(if selected { accent } else { theme::TEXT });
            if ui.selectable_label(selected, txt).clicked() {
                select = Some(i);
            }
        }
    });
    if let Some(i) = select {
        app.breathing.select(i);
    }

    // 选择既定，取当前预设（'static，不借用 app）
    let pat = app.breathing.pattern();
    let accent = rgb(pat.accent);

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        badge(ui, pat.goal, accent);
        ui.label(RichText::new(pat.tagline).size(13.0).color(theme::TEXT_WEAK));
    });

    // ② 完成检测：达到目标轮数则收尾并记账（写明细 + 计入修为）
    if app.breathing.running
        && app.breathing.completed_cycles() >= app.breathing.target_cycles
        && !app.breathing.session_logged
    {
        let secs = app.breathing.elapsed().round().max(0.0) as u32;
        let cycles = app.breathing.target_cycles;
        let key = pat.key;
        app.breathing.mark_finished();
        app.finish_breathing(key, cycles, secs);
    }

    // ③ 切拍提示音：相位边界变化时发一声（经调度线程播放）
    let phase = app.breathing.current();
    if app.breathing.running && app.breathing.sound_on {
        let bound = (phase.cycle, phase.phase_index);
        if app.breathing.last_boundary.is_some() && app.breathing.last_boundary != Some(bound) {
            let intensity = match phase.kind {
                PhaseKind::Inhale => Intensity::Medium,
                _ => Intensity::Soft,
            };
            app.send(Command::Beep(intensity));
        }
        app.breathing.last_boundary = Some(bound);
    }

    ui.add_space(10.0);

    // ④ 动画圆圈 + 进度 + 控制，包一层卡片聚焦
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, accent.linear_multiply(0.4)))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(16.0, 14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            draw_pacer_circle(
                ui,
                accent,
                phase,
                app.breathing.running,
                app.breathing.elapsed() > 0.0,
                app.breathing.session_logged,
            );
            ui.add_space(10.0);

            // 轮数进度条
            let completed = app.breathing.completed_cycles().min(app.breathing.target_cycles);
            let ratio = completed as f32 / app.breathing.target_cycles.max(1) as f32;
            ui.add(
                egui::ProgressBar::new(ratio.clamp(0.0, 1.0))
                    .desired_width(ui.available_width().min(360.0))
                    .text(
                        RichText::new(format!("{} / {} 轮", completed, app.breathing.target_cycles))
                            .size(12.0),
                    ),
            );
            ui.add_space(10.0);

            controls(app, ui, accent);
        });

    // 依据来源
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("依据：").size(12.0).color(theme::TEXT_WEAK));
        ui.hyperlink_to(
            RichText::new(pat.source_name).size(12.0),
            pat.source_url,
        );
    });
    ui.add_space(2.0);
    ui.add(
        egui::Label::new(RichText::new(pat.note).size(12.0).color(theme::TEXT_WEAK)).wrap(),
    );

    // 运行中持续请求重绘，保证动画连续
    if app.breathing.running {
        ui.ctx().request_repaint();
    }
}

/// 画呼吸圆圈：外圈参考环 + 随相位缩放的主圆 + 圆心相位名/倒计时。
/// 不直接依赖 App，运行/已开始/已完成三态由调用方传入，便于单测。
fn draw_pacer_circle(
    ui: &mut egui::Ui,
    accent: Color32,
    phase: PhaseAt,
    running: bool,
    started: bool,
    finished: bool,
) {
    let (rect, _resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 240.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let max_r = 100.0_f32;
    let min_r = 38.0_f32;
    let r = min_r + (max_r - min_r) * phase.radius_t;

    // 外圈参考环（最大半径）
    painter.circle_stroke(
        center,
        max_r,
        egui::Stroke::new(1.5, accent.linear_multiply(0.22)),
    );
    // 主圆：越涨越亮
    let glow = 0.26 + 0.34 * phase.radius_t;
    painter.circle_filled(center, r, accent.linear_multiply(glow));
    painter.circle_stroke(center, r, egui::Stroke::new(2.0, accent));

    // 圆心文字
    let label = if finished {
        "完成 ✓"
    } else if running || started {
        phase.kind.label()
    } else {
        "准备"
    };
    painter.text(
        center - egui::vec2(0.0, 14.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(24.0),
        theme::TEXT,
    );
    if (running || started) && !finished {
        let secs = phase.phase_remaining.ceil().max(0.0) as i32;
        painter.text(
            center + egui::vec2(0.0, 20.0),
            egui::Align2::CENTER_CENTER,
            format!("{}", secs),
            egui::FontId::proportional(30.0),
            accent,
        );
    }
}

fn controls(app: &mut App, ui: &mut egui::Ui, accent: Color32) {
    ui.horizontal(|ui| {
        // 目标轮数调节
        if ui.small_button("−").clicked() && app.breathing.target_cycles > 1 {
            app.breathing.target_cycles -= 1;
        }
        ui.label(
            RichText::new(format!("目标 {} 轮", app.breathing.target_cycles))
                .size(13.0)
                .color(theme::TEXT_WEAK),
        );
        if ui.small_button("+").clicked() && app.breathing.target_cycles < 60 {
            app.breathing.target_cycles += 1;
        }

        ui.separator();

        if app.breathing.running {
            if ui.button("⏸ 暂停").clicked() {
                app.breathing.pause();
            }
        } else {
            let label = if app.breathing.session_logged {
                "▶ 再来一次"
            } else if app.breathing.elapsed() > 0.0 {
                "▶ 继续"
            } else {
                "▶ 开始"
            };
            if ui
                .button(RichText::new(label).strong().color(accent))
                .clicked()
            {
                app.breathing.start();
            }
        }
        if ui.button("⟲ 重置").clicked() {
            app.breathing.reset();
        }

        ui.separator();

        let mut snd = app.breathing.sound_on;
        if ui.checkbox(&mut snd, "提示音").changed() {
            app.breathing.sound_on = snd;
        }
    });
}

/// 今日呼吸统计横幅
fn stats_banner(app: &App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        stat_card(
            ui,
            "🌬",
            format!("{}", app.breathing_count),
            "今日次数",
            theme::INFO,
        );
        stat_card(
            ui,
            "⏱",
            format!("{} 分", app.breathing_secs / 60),
            "今日时长",
            theme::ACCENT,
        );
        stat_card(
            ui,
            "🔥",
            format!("{} 天", app.breathing_streak),
            "连续练习",
            theme::WARN,
        );
    });
}

/// 呼吸法门图文库：卡片 + 「跟练」联动 + 打卡（计入修为）
fn library_section(app: &mut App, ui: &mut egui::Ui) {
    let accent = rgb(PracticeCategory::Breathing.accent());
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("呼吸法门 · 图文详解")
                .size(16.0)
                .strong()
                .color(accent),
        );
        let n = app.practices.by_category(PracticeCategory::Breathing).len();
        ui.label(
            RichText::new(format!("· {} 法", n))
                .size(12.5)
                .color(theme::TEXT_WEAK),
        );
    });
    ui.add_space(8.0);

    let logged: HashSet<String> = app.cultivation.today_logged.iter().cloned().collect();
    let practices = app.practices.by_category(PracticeCategory::Breathing);
    let mut pending_checkin: Option<String> = None;
    let mut start_pattern: Option<usize> = None;

    for practice in practices {
        practice_card(ui, practice, accent);
        ui.horizontal(|ui| {
            // 若该法门对应某练习台预设，给一个「跟练」入口
            if let Some(idx) = pattern_index_for(&practice.title) {
                if ui
                    .button(RichText::new("▶ 跟练").size(13.0).strong().color(accent))
                    .clicked()
                {
                    start_pattern = Some(idx);
                }
            }
            let done = logged.contains(&practice.title);
            if checkin_button(ui, done, accent) {
                pending_checkin = Some(practice.title.clone());
            }
        });
        ui.add_space(12.0);
    }

    // practices 的不可变借用已随 for 消费结束，安全地写回
    if let Some(idx) = start_pattern {
        app.breathing.select(idx);
        app.breathing.start();
    }
    if let Some(title) = pending_checkin {
        app.log_practice("breathing", title);
    }
}

/// 图文法门标题 → 练习台预设下标（按关键词匹配，无对应则 None）
fn pattern_index_for(title: &str) -> Option<usize> {
    let key = if title.contains("4-7-8") {
        "478"
    } else if title.contains("箱式") || title.contains("四方") {
        "box"
    } else if title.contains("生理叹息") {
        "sigh"
    } else if title.contains("共振") || title.contains("相干") {
        "coherent"
    } else if title.contains("腹式") || title.contains("膈肌") {
        "belly"
    } else if title.contains("延长呼气") {
        "exhale"
    } else {
        return None;
    };
    BREATHING_PATTERNS.iter().position(|p| p.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breathing::phase_at;

    #[test]
    fn library_titles_link_to_pacer_patterns() {
        // 图文库现代法门标题必须能映射到练习台预设（否则「跟练」失效）
        for title in [
            "4-7-8 呼吸法：延长呼气助眠",
            "箱式呼吸：四方等长稳心神",
            "生理叹息：双吸长呼快速减压",
            "共振呼吸：每分钟六息",
            "腹式深呼吸：膈肌带动",
            "延长呼气：呼倍于吸",
        ] {
            assert!(pattern_index_for(title).is_some(), "跟练映射缺失: {title}");
        }
        // 传统觉知类法门没有固定节律，不提供跟练
        assert!(pattern_index_for("安那般那：知息出入").is_none());
    }

    #[test]
    fn pacer_circle_renders_all_phases_without_panic() {
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
                let accent = Color32::from_rgb(120, 190, 180);
                let pat = &BREATHING_PATTERNS[1]; // 4-7-8：含吸/屏/呼三态
                // 运行中，遍历各相位时刻
                for t in [0.0f32, 2.0, 4.0, 7.0, 11.0, 15.0, 19.0] {
                    draw_pacer_circle(ui, accent, phase_at(pat, t), true, true, false);
                }
                // 未开始 / 已完成两态
                draw_pacer_circle(ui, accent, phase_at(pat, 0.0), false, false, false);
                draw_pacer_circle(ui, accent, phase_at(pat, 0.0), false, true, true);
            });
        });
    }
}
