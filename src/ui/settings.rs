// 设置面板
// 所有可调整项都从 Arc<Mutex<Config>> 读写，保存时持久化到 config.toml

use eframe::egui::{self, RichText};

use crate::app::App;
use crate::config;

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("⚙ 设置");
    ui.add_space(8.0);

    let mut cfg = app.config.lock().unwrap().clone();
    let mut dirty = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // 一、提醒开关 + 间隔
        ui.label(RichText::new("提醒开关与周期").strong());
        ui.add_space(4.0);
        egui::Grid::new("reminder-grid")
            .num_columns(3)
            .spacing([16.0, 8.0])
            .min_col_width(110.0)
            .show(ui, |ui| {
                dirty |= toggle_row(ui, "👁 护眼（20-20-20）", &mut cfg.reminders.enabled.eyes);
                dirty |= minutes_slider(ui, &mut cfg.reminders.eyes_interval_sec, 5, 60);
                ui.end_row();

                dirty |= toggle_row(ui, "🚶 起身舒展", &mut cfg.reminders.enabled.stand);
                dirty |= minutes_slider(ui, &mut cfg.reminders.stand_interval_sec, 10, 90);
                ui.end_row();

                dirty |= toggle_row(ui, "💧 喝水", &mut cfg.reminders.enabled.water);
                dirty |= minutes_slider(ui, &mut cfg.reminders.water_interval_sec, 15, 120);
                ui.end_row();

                dirty |= toggle_row(ui, "🦴 颈椎活动", &mut cfg.reminders.enabled.neck);
                dirty |= minutes_slider(ui, &mut cfg.reminders.neck_interval_sec, 20, 120);
                ui.end_row();

                dirty |= toggle_row(ui, "🍅 番茄钟", &mut cfg.reminders.enabled.pomodoro);
                ui.horizontal(|ui| {
                    let f_mins = (cfg.reminders.pomodoro_focus_sec / 60) as u32;
                    let mut f = f_mins;
                    if ui.add(egui::DragValue::new(&mut f).range(15..=120).suffix(" min 专注")).changed() {
                        cfg.reminders.pomodoro_focus_sec = (f as u64) * 60;
                        dirty = true;
                    }
                    let b_mins = (cfg.reminders.pomodoro_break_sec / 60) as u32;
                    let mut b = b_mins;
                    if ui.add(egui::DragValue::new(&mut b).range(3..=30).suffix(" min 休息")).changed() {
                        cfg.reminders.pomodoro_break_sec = (b as u64) * 60;
                        dirty = true;
                    }
                });
                ui.end_row();

                dirty |= toggle_row(ui, "🛌 大休息（强制）", &mut cfg.reminders.enabled.big_break);
                ui.horizontal(|ui| {
                    let mut iv = (cfg.reminders.big_break_interval_sec / 60) as u32;
                    if ui.add(egui::DragValue::new(&mut iv).range(45..=180).suffix(" min 周期")).changed() {
                        cfg.reminders.big_break_interval_sec = (iv as u64) * 60;
                        dirty = true;
                    }
                    let mut du = (cfg.reminders.big_break_duration_sec / 60) as u32;
                    if ui.add(egui::DragValue::new(&mut du).range(2..=15).suffix(" min 时长")).changed() {
                        cfg.reminders.big_break_duration_sec = (du as u64) * 60;
                        dirty = true;
                    }
                });
                ui.end_row();

                dirty |= toggle_row(ui, "🍱 午餐", &mut cfg.reminders.enabled.lunch);
                if ui.text_edit_singleline(&mut cfg.reminders.lunch_time).changed() {
                    dirty = true;
                }
                ui.end_row();

                dirty |= toggle_row(ui, "🌙 睡眠提醒", &mut cfg.reminders.enabled.sleep);
                if ui.text_edit_singleline(&mut cfg.reminders.sleep_time).changed() {
                    dirty = true;
                }
                ui.end_row();

                dirty |= toggle_row(ui, "🏁 下班建议", &mut cfg.reminders.enabled.off_work);
                let mut hours = (cfg.reminders.off_work_total_sec / 3600) as u32;
                if ui.add(egui::DragValue::new(&mut hours).range(4..=12).suffix(" 小时累计")).changed() {
                    cfg.reminders.off_work_total_sec = (hours as u64) * 3600;
                    dirty = true;
                }
                ui.end_row();
            });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // 二、通知与音效
        ui.label(RichText::new("通知与音效").strong());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.checkbox(&mut cfg.general.desktop_notify, "桌面通知").changed() {
                dirty = true;
            }
            if ui.button("🔔 测试通知").clicked() {
                app.send(crate::scheduler::Command::TestNotify);
            }
        });
        if ui.checkbox(&mut cfg.general.sound_enabled, "声音提示").changed() {
            dirty = true;
        }
        ui.horizontal(|ui| {
            ui.label("音量");
            if ui.add(egui::Slider::new(&mut cfg.general.volume, 0.0..=1.0).text("0~1")).changed() {
                dirty = true;
            }
            if ui.button("🔊 试听").clicked() {
                app.send(crate::scheduler::Command::TestSound);
            }
        });

        ui.add_space(12.0);
        ui.label(RichText::new("勿扰时段（除强制大休息外不提醒）").strong());
        ui.horizontal(|ui| {
            ui.label("开始");
            if ui.text_edit_singleline(&mut cfg.general.quiet_start).changed() {
                dirty = true;
            }
            ui.label("结束");
            if ui.text_edit_singleline(&mut cfg.general.quiet_end).changed() {
                dirty = true;
            }
        });

        ui.add_space(12.0);
        ui.label(RichText::new("通知最小间隔（微提醒错开，0=不限）").strong());
        ui.horizontal(|ui| {
            let mut gap = (cfg.general.min_notify_gap_sec / 60) as u32;
            if ui
                .add(egui::DragValue::new(&mut gap).range(0..=30).suffix(" 分钟"))
                .changed()
            {
                cfg.general.min_notify_gap_sec = (gap as u64) * 60;
                dirty = true;
            }
            ui.label(
                RichText::new("该间隔内护眼/起身/喝水/颈椎不会重复打扰")
                    .weak()
                    .small(),
            );
        });

        ui.add_space(12.0);
        ui.label(RichText::new("跳过冷却（强制休息窗）").strong());
        let mut cd = cfg.general.skip_cooldown_sec as u32;
        if ui.add(egui::DragValue::new(&mut cd).range(0..=60).suffix(" 秒")).changed() {
            cfg.general.skip_cooldown_sec = cd as u64;
            dirty = true;
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // 三、保存按钮
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new(RichText::new("💾 保存设置").size(15.0)).min_size(egui::vec2(120.0, 32.0)))
                .clicked()
            {
                let new_cfg = cfg.clone();
                *app.config.lock().unwrap() = new_cfg.clone();
                if let Err(e) = config::save(&new_cfg) {
                    app.error_msg = Some(format!("保存失败: {e}"));
                } else {
                    app.error_msg = Some("已保存".to_string());
                }
            }
            ui.add(
                egui::Label::new(
                    RichText::new(format!("配置位置: {}", cfg.paths.config_file.display()))
                        .weak()
                        .small(),
                )
                .wrap(),
            );
        });
    });

    // 把临时改动同步回去（即便没点保存也保持运行中可见），
    // 但持久化只在点保存时执行
    if dirty {
        let mut held = app.config.lock().unwrap();
        // 保留路径
        let paths = held.paths.clone();
        *held = cfg;
        held.paths = paths;
    }
}

fn toggle_row(ui: &mut egui::Ui, label: &str, flag: &mut bool) -> bool {
    let before = *flag;
    ui.checkbox(flag, label);
    before != *flag
}

fn minutes_slider(ui: &mut egui::Ui, secs: &mut u64, min_min: u32, max_min: u32) -> bool {
    let mut mins = (*secs / 60) as u32;
    let resp = ui.add(egui::DragValue::new(&mut mins).range(min_min..=max_min).suffix(" min"));
    if resp.changed() {
        *secs = (mins as u64) * 60;
        return true;
    }
    false
}
