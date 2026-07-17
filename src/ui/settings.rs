// 设置面板：四页签 + 独立草稿 + 显式保存

use eframe::egui::{self, RichText};

use crate::app::App;
use crate::config::{self, Config};
#[cfg(debug_assertions)]
use crate::reminders::ReminderKind;
use crate::scheduler::Command;
use crate::ui::{theme, widgets};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Rhythm,
    Schedule,
    Notification,
    Application,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            SettingsTab::Rhythm => "提醒节奏",
            SettingsTab::Schedule => "日程提醒",
            SettingsTab::Notification => "通知与勿扰",
            SettingsTab::Application => "应用信息",
        }
    }

    fn all() -> &'static [SettingsTab] {
        const ALL: [SettingsTab; 4] = [
            SettingsTab::Rhythm,
            SettingsTab::Schedule,
            SettingsTab::Notification,
            SettingsTab::Application,
        ];
        &ALL
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeErrors {
    pub lunch: Option<&'static str>,
    pub sleep: Option<&'static str>,
    pub quiet_start: Option<&'static str>,
    pub quiet_end: Option<&'static str>,
}

impl TimeErrors {
    fn is_empty(&self) -> bool {
        self.lunch.is_none()
            && self.sleep.is_none()
            && self.quiet_start.is_none()
            && self.quiet_end.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub tab: SettingsTab,
    pub draft: Config,
    pub dirty: bool,
    pub errors: TimeErrors,
}

impl SettingsState {
    pub fn new(config: &Config) -> Self {
        Self {
            tab: SettingsTab::Rhythm,
            draft: config.clone(),
            dirty: false,
            errors: TimeErrors::default(),
        }
    }

    pub fn reset(&mut self, config: &Config) {
        self.draft = config.clone();
        self.dirty = false;
        self.errors = TimeErrors::default();
    }

    fn validate(&mut self) -> bool {
        const MESSAGE: &str = "请输入 HH:MM 格式的有效时间";
        self.errors = TimeErrors {
            lunch: (!is_strict_hhmm(&self.draft.reminders.lunch_time)).then_some(MESSAGE),
            sleep: (!is_strict_hhmm(&self.draft.reminders.sleep_time)).then_some(MESSAGE),
            quiet_start: (!is_strict_hhmm(&self.draft.general.quiet_start)).then_some(MESSAGE),
            quiet_end: (!is_strict_hhmm(&self.draft.general.quiet_end)).then_some(MESSAGE),
        };
        self.errors.is_empty()
    }
}

fn is_strict_hhmm(value: &str) -> bool {
    let value = value.trim();
    let bytes = value.as_bytes();
    value.len() == 5
        && bytes[2] == b':'
        && bytes[..2].iter().all(u8::is_ascii_digit)
        && bytes[3..].iter().all(u8::is_ascii_digit)
        && config::parse_hhmm(value).is_some()
}

#[derive(Debug, Clone, Copy)]
enum SettingsAction {
    Save,
    Cancel,
}

#[derive(Debug, Clone, Copy)]
enum SettingsCommand {
    TestNotify,
    TestSound,
    #[cfg(debug_assertions)]
    Trigger(ReminderKind),
}

pub fn render(app: &mut App, ui: &mut egui::Ui) {
    widgets::page_header(ui, "设置", "调整提醒节奏、日程与通知方式；保存成功后才会应用。 ");
    ui.add_space(14.0);

    let mut action = None;
    let mut command = None;
    {
        let state = &mut app.settings;

        ui.horizontal_wrapped(|ui| {
            for tab in SettingsTab::all() {
                let selected = state.tab == *tab;
                let text = RichText::new(tab.label()).size(13.5).color(if selected {
                    theme::ACCENT
                } else {
                    theme::TEXT_WEAK
                });
                if ui.selectable_label(selected, text).clicked() {
                    state.tab = *tab;
                }
            }
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        let body_height = (ui.available_height() - 58.0).max(180.0);
        egui::ScrollArea::vertical()
            .max_height(body_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let changed = match state.tab {
                    SettingsTab::Rhythm => rhythm_tab(&mut state.draft, ui),
                    SettingsTab::Schedule => schedule_tab(&mut state.draft, &state.errors, ui),
                    SettingsTab::Notification => notification_tab(
                        &mut state.draft,
                        &state.errors,
                        ui,
                        &mut command,
                    ),
                    SettingsTab::Application => application_tab(&state.draft, ui, &mut command),
                };
                if changed {
                    state.dirty = true;
                    state.errors = TimeErrors::default();
                }
                ui.add_space(10.0);
            });

        ui.separator();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if state.dirty {
                widgets::status_badge(ui, "有未保存更改", theme::WARN);
            } else {
                ui.label(RichText::new("设置已同步").size(12.5).color(theme::TEXT_WEAK));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        state.dirty,
                        egui::Button::new(RichText::new("保存设置").strong())
                            .fill(theme::ACCENT.linear_multiply(0.35)),
                    )
                    .clicked()
                {
                    action = Some(SettingsAction::Save);
                }
                if ui
                    .add_enabled(state.dirty, egui::Button::new("取消更改"))
                    .clicked()
                {
                    action = Some(SettingsAction::Cancel);
                }
            });
        });
    }

    if let Some(command) = command {
        match command {
            SettingsCommand::TestNotify => app.send(Command::TestNotify),
            SettingsCommand::TestSound => app.send(Command::TestSound),
            #[cfg(debug_assertions)]
            SettingsCommand::Trigger(kind) => app.send(Command::TriggerNow(kind)),
        }
    }

    match action {
        Some(SettingsAction::Cancel) => {
            let live = app
                .config
                .lock()
                .expect("配置锁不应被持有线程破坏")
                .clone();
            app.settings.reset(&live);
            app.show_success("已取消未保存的更改");
        }
        Some(SettingsAction::Save) => save_draft(app),
        None => {}
    }
}

fn rhythm_tab(cfg: &mut Config, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    setting_group(
        ui,
        "微休息",
        "让护眼、起身、补水与颈椎活动错开出现，减少连续打扰。",
        |ui| {
            changed |= interval_row(
                ui,
                "护眼 · 20-20-20",
                "👁",
                &mut cfg.reminders.enabled.eyes,
                &mut cfg.reminders.eyes_interval_sec,
                5,
                60,
            );
            changed |= interval_row(
                ui,
                "起身舒展",
                "↟",
                &mut cfg.reminders.enabled.stand,
                &mut cfg.reminders.stand_interval_sec,
                10,
                90,
            );
            changed |= interval_row(
                ui,
                "喝水",
                "◒",
                &mut cfg.reminders.enabled.water,
                &mut cfg.reminders.water_interval_sec,
                15,
                120,
            );
            changed |= interval_row(
                ui,
                "颈椎活动",
                "⌁",
                &mut cfg.reminders.enabled.neck,
                &mut cfg.reminders.neck_interval_sec,
                20,
                120,
            );
        },
    );

    ui.add_space(12.0);
    setting_group(ui, "专注与大休息", "分别控制番茄节奏和强制大休息路线。", |ui| {
        changed |= toggle_row(ui, "番茄钟", "专注与短休息循环", &mut cfg.reminders.enabled.pomodoro);
        ui.indent("pomodoro-values", |ui| {
            ui.horizontal_wrapped(|ui| {
                changed |= minute_drag(ui, "专注", &mut cfg.reminders.pomodoro_focus_sec, 15, 120);
                changed |= minute_drag(ui, "休息", &mut cfg.reminders.pomodoro_break_sec, 3, 30);
            });
        });
        ui.add_space(6.0);
        changed |= toggle_row(ui, "大休息", "到点弹出分段跟练窗口", &mut cfg.reminders.enabled.big_break);
        ui.indent("big-break-values", |ui| {
            ui.horizontal_wrapped(|ui| {
                changed |= minute_drag(ui, "周期", &mut cfg.reminders.big_break_interval_sec, 45, 180);
                changed |= minute_drag(ui, "时长", &mut cfg.reminders.big_break_duration_sec, 2, 15);
            });
        });
    });
    changed
}

fn schedule_tab(cfg: &mut Config, errors: &TimeErrors, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    setting_group(ui, "固定时间", "按本地时间提醒午餐和睡眠。", |ui| {
        changed |= time_row(
            ui,
            "午餐提醒",
            "每日一次",
            &mut cfg.reminders.enabled.lunch,
            &mut cfg.reminders.lunch_time,
            errors.lunch,
        );
        changed |= time_row(
            ui,
            "睡眠提醒",
            "每日一次",
            &mut cfg.reminders.enabled.sleep,
            &mut cfg.reminders.sleep_time,
            errors.sleep,
        );
    });

    ui.add_space(12.0);
    setting_group(ui, "工作总量", "达到累计工作时长后给出下班建议。", |ui| {
        changed |= toggle_row(
            ui,
            "下班建议",
            "当前工作会话累计达到阈值时提醒",
            &mut cfg.reminders.enabled.off_work,
        );
        let mut hours = (cfg.reminders.off_work_total_sec / 3600) as u32;
        ui.indent("off-work-hours", |ui| {
            if ui
                .add(egui::DragValue::new(&mut hours).range(4..=12).suffix(" 小时"))
                .changed()
            {
                cfg.reminders.off_work_total_sec = (hours as u64) * 3600;
                changed = true;
            }
        });
    });
    changed
}

fn notification_tab(
    cfg: &mut Config,
    errors: &TimeErrors,
    ui: &mut egui::Ui,
    command: &mut Option<SettingsCommand>,
) -> bool {
    let mut changed = false;
    setting_group(ui, "通知渠道", "测试操作不会修改设置。", |ui| {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut cfg.general.desktop_notify, "桌面通知").changed() {
                changed = true;
            }
            if ui.button("测试通知").clicked() {
                *command = Some(SettingsCommand::TestNotify);
            }
        });
        ui.horizontal(|ui| {
            if ui.checkbox(&mut cfg.general.sound_enabled, "声音提示").changed() {
                changed = true;
            }
            ui.label(RichText::new("音量").color(theme::TEXT_WEAK));
            if ui
                .add(egui::Slider::new(&mut cfg.general.volume, 0.0..=1.0).show_value(false))
                .changed()
            {
                changed = true;
            }
            if ui.button("试听").clicked() {
                *command = Some(SettingsCommand::TestSound);
            }
        });
    });

    ui.add_space(12.0);
    setting_group(ui, "勿扰与错开", "大休息不受勿扰时段限制。", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("勿扰时段");
            changed |= time_input(ui, &mut cfg.general.quiet_start, errors.quiet_start);
            ui.label(RichText::new("至").color(theme::TEXT_WEAK));
            changed |= time_input(ui, &mut cfg.general.quiet_end, errors.quiet_end);
        });
        if errors.quiet_start.is_some() || errors.quiet_end.is_some() {
            ui.label(
                RichText::new("请输入 HH:MM 格式的有效勿扰时间")
                    .size(12.0)
                    .color(theme::DANGER),
            );
        }
        ui.label(
            RichText::new("开始与结束相同表示关闭勿扰")
                .size(12.0)
                .color(theme::TEXT_WEAK),
        );
        ui.add_space(8.0);
        changed |= minute_drag(
            ui,
            "微提醒最小间隔",
            &mut cfg.general.min_notify_gap_sec,
            0,
            30,
        );
        let mut seconds = cfg.general.skip_cooldown_sec as u32;
        ui.horizontal(|ui| {
            ui.label("大休息跳过冷却");
            if ui
                .add(egui::DragValue::new(&mut seconds).range(0..=60).suffix(" 秒"))
                .changed()
            {
                cfg.general.skip_cooldown_sec = seconds as u64;
                changed = true;
            }
        });
    });
    changed
}

fn application_tab(
    cfg: &Config,
    ui: &mut egui::Ui,
    _command: &mut Option<SettingsCommand>,
) -> bool {
    setting_group(ui, "Lifetime 健康助手", "程序员与久坐人群的本地健康提醒工具。", |ui| {
        info_row(ui, "版本", env!("CARGO_PKG_VERSION"));
        info_row(ui, "数据策略", "完全保存在本机，不联网");
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                RichText::new(format!("配置文件  {}", cfg.paths.config_file.display()))
                    .size(12.0)
                    .color(theme::TEXT_WEAK),
            )
            .wrap(),
        );
        ui.add(
            egui::Label::new(
                RichText::new(format!("统计数据  {}", cfg.paths.db_file.display()))
                    .size(12.0)
                    .color(theme::TEXT_WEAK),
            )
            .wrap(),
        );
    });

    #[cfg(debug_assertions)]
    {
        ui.add_space(12.0);
        setting_group(ui, "开发工具", "仅调试构建可见。", |ui| {
            ui.collapsing("手动触发提醒", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for kind in ReminderKind::all() {
                        if ui.small_button(kind.label()).clicked() {
                            *_command = Some(SettingsCommand::Trigger(*kind));
                        }
                    }
                });
            });
        });
    }
    false
}

fn save_draft(app: &mut App) {
    if !app.settings.validate() {
        app.settings.tab = if app.settings.errors.lunch.is_some() || app.settings.errors.sleep.is_some() {
            SettingsTab::Schedule
        } else {
            SettingsTab::Notification
        };
        app.show_error("部分时间格式无效，请修正后再保存");
        return;
    }

    let draft = app.settings.draft.clone();
    match config::save(&draft) {
        Ok(()) => {
            *app.config
                .lock()
                .expect("配置锁不应被持有线程破坏") = draft;
            app.settings.dirty = false;
            app.show_success("设置已保存并生效");
        }
        Err(error) => app.show_error(format!("保存设置失败：{error}")),
    }
}

fn setting_group(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(egui::Stroke::new(1.0, theme::STROKE))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            widgets::section_header(ui, title, None);
            ui.label(RichText::new(subtitle).size(12.0).color(theme::TEXT_WEAK));
            ui.add_space(10.0);
            add_contents(ui);
        });
}

fn interval_row(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    enabled: &mut bool,
    seconds: &mut u64,
    min_minutes: u32,
    max_minutes: u32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).color(theme::ACCENT));
        if ui.checkbox(enabled, label).changed() {
            changed = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed |= minute_value(ui, seconds, min_minutes, max_minutes);
        });
    });
    changed
}

fn toggle_row(ui: &mut egui::Ui, title: &str, hint: &str, enabled: &mut bool) -> bool {
    let before = *enabled;
    ui.horizontal(|ui| {
        ui.checkbox(enabled, title);
        ui.label(RichText::new(hint).size(12.0).color(theme::TEXT_WEAK));
    });
    before != *enabled
}

fn minute_drag(
    ui: &mut egui::Ui,
    label: &str,
    seconds: &mut u64,
    min_minutes: u32,
    max_minutes: u32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(theme::TEXT_WEAK));
        changed = minute_value(ui, seconds, min_minutes, max_minutes);
    });
    changed
}

fn minute_value(
    ui: &mut egui::Ui,
    seconds: &mut u64,
    min_minutes: u32,
    max_minutes: u32,
) -> bool {
    let mut minutes = (*seconds / 60) as u32;
    if ui
        .add(
            egui::DragValue::new(&mut minutes)
                .range(min_minutes..=max_minutes)
                .suffix(" 分钟"),
        )
        .changed()
    {
        *seconds = (minutes as u64) * 60;
        true
    } else {
        false
    }
}

fn time_row(
    ui: &mut egui::Ui,
    title: &str,
    hint: &str,
    enabled: &mut bool,
    value: &mut String,
    error: Option<&str>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.checkbox(enabled, title).changed() {
            changed = true;
        }
        ui.label(RichText::new(hint).size(12.0).color(theme::TEXT_WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed |= time_input(ui, value, error);
        });
    });
    if let Some(error) = error {
        ui.label(RichText::new(error).size(12.0).color(theme::DANGER));
    }
    changed
}

fn time_input(ui: &mut egui::Ui, value: &mut String, error: Option<&str>) -> bool {
    let stroke = error.map(|_| egui::Stroke::new(1.0, theme::DANGER));
    let response = egui::Frame::none()
        .stroke(stroke.unwrap_or(egui::Stroke::NONE))
        .rounding(5.0)
        .show(ui, |ui| {
            ui.add_sized([76.0, 28.0], egui::TextEdit::singleline(value).hint_text("HH:MM"))
        })
        .inner;
    response.changed()
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(theme::TEXT_WEAK));
        ui.label(RichText::new(value).color(theme::TEXT));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_validation_reports_each_invalid_field() {
        let mut state = SettingsState::new(&Config::default());
        state.draft.reminders.lunch_time = "25:00".into();
        state.draft.reminders.sleep_time = "bad".into();
        state.draft.general.quiet_start = "9:00".into();
        state.draft.general.quiet_end = String::new();

        assert!(!state.validate());
        assert!(state.errors.lunch.is_some());
        assert!(state.errors.sleep.is_some());
        assert!(state.errors.quiet_start.is_some());
        assert!(state.errors.quiet_end.is_some());
    }

    #[test]
    fn equal_quiet_hours_are_valid_and_reset_restores_live_config() {
        let mut live = Config::default();
        live.general.quiet_start = "00:00".into();
        live.general.quiet_end = "00:00".into();
        let mut state = SettingsState::new(&live);
        assert!(state.validate());

        state.draft.general.volume = 0.1;
        state.dirty = true;
        state.reset(&live);
        assert!(!state.dirty);
        assert_eq!(state.draft.general.volume, live.general.volume);
    }
}
