// 配置层 —— 用 dirs crate 拿跨平台路径，TOML 持久化
//
// 配置文件位置：
//   Linux:   ~/.config/lifetime/config.toml
//   macOS:   ~/Library/Application Support/lifetime/config.toml
//   Windows: %APPDATA%\lifetime\config.toml

use anyhow::{Context, Result};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::reminders::ReminderKind;

const APP_DIR: &str = "lifetime";
const CONFIG_FILE: &str = "config.toml";
const DB_FILE: &str = "lifetime.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub reminders: ReminderConfig,
    pub general: GeneralConfig,
    #[serde(skip)]
    pub paths: Paths,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderConfig {
    /// 护眼（20-20-20）周期，秒
    pub eyes_interval_sec: u64,
    /// 起身微休息周期
    pub stand_interval_sec: u64,
    /// 喝水周期
    pub water_interval_sec: u64,
    /// 颈椎活动周期
    pub neck_interval_sec: u64,
    /// 番茄钟工作时长
    pub pomodoro_focus_sec: u64,
    /// 番茄钟休息时长
    pub pomodoro_break_sec: u64,
    /// 大休息触发周期（连续工作 N 分钟后弹模态）
    pub big_break_interval_sec: u64,
    /// 大休息时长（模态窗倒计时）
    pub big_break_duration_sec: u64,
    /// 累计多长建议下班
    pub off_work_total_sec: u64,
    /// 午餐提醒时间（HH:MM）
    pub lunch_time: String,
    /// 睡眠提醒时间
    pub sleep_time: String,

    /// 开关：每项是否启用
    pub enabled: ReminderToggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderToggle {
    pub eyes: bool,
    pub stand: bool,
    pub water: bool,
    pub neck: bool,
    pub pomodoro: bool,
    pub big_break: bool,
    pub lunch: bool,
    pub off_work: bool,
    pub sleep: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// 是否按工作时段自动开始/停止会话
    #[serde(default = "default_auto_schedule")]
    pub auto_schedule: bool,
    /// 每日自动开始时间（HH:MM）
    #[serde(default = "default_work_start")]
    pub work_start: String,
    /// 每日自动停止时间（HH:MM）
    #[serde(default = "default_work_end")]
    pub work_end: String,
    /// 桌面通知是否开启
    pub desktop_notify: bool,
    /// 声音提示是否开启
    pub sound_enabled: bool,
    /// 音量 0.0 ~ 1.0
    pub volume: f32,
    /// 强制休息跳过按钮冷却（秒）
    pub skip_cooldown_sec: u64,
    /// 勿扰开始 HH:MM（在此区间不提醒，除大休息外）
    pub quiet_start: String,
    /// 勿扰结束
    pub quiet_end: String,
    /// 微提醒（护眼/起身/喝水/颈椎）的全局最小间隔，秒；0 表示不限制。
    /// 用于错开：该间隔内不会出现两次微提醒。
    #[serde(default = "default_min_gap")]
    pub min_notify_gap_sec: u64,
}

fn default_min_gap() -> u64 {
    10 * 60
}

fn default_auto_schedule() -> bool {
    true
}

fn default_work_start() -> String {
    "09:00".to_string()
}

fn default_work_end() -> String {
    "19:00".to_string()
}

#[derive(Debug, Clone, Default)]
pub struct Paths {
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
    pub db_file: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            reminders: ReminderConfig::default(),
            general: GeneralConfig::default(),
            paths: Paths::default(),
        }
    }
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            eyes_interval_sec: 20 * 60,
            stand_interval_sec: 30 * 60,
            water_interval_sec: 45 * 60,
            neck_interval_sec: 60 * 60,
            pomodoro_focus_sec: 50 * 60,
            pomodoro_break_sec: 10 * 60,
            big_break_interval_sec: 90 * 60,
            big_break_duration_sec: 5 * 60,
            off_work_total_sec: 8 * 3600,
            lunch_time: "12:00".to_string(),
            sleep_time: "22:30".to_string(),
            enabled: ReminderToggle::default(),
        }
    }
}

impl Default for ReminderToggle {
    fn default() -> Self {
        Self {
            eyes: true,
            stand: true,
            water: true,
            neck: true,
            pomodoro: true,
            big_break: true,
            lunch: true,
            off_work: true,
            sleep: true,
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            auto_schedule: default_auto_schedule(),
            work_start: default_work_start(),
            work_end: default_work_end(),
            desktop_notify: true,
            sound_enabled: true,
            volume: 0.6,
            skip_cooldown_sec: 15,
            quiet_start: "12:00".to_string(),
            quiet_end: "13:00".to_string(),
            min_notify_gap_sec: default_min_gap(),
        }
    }
}

impl ReminderConfig {
    /// 根据 ReminderKind 取对应启用开关
    pub fn is_enabled(&self, kind: ReminderKind) -> bool {
        match kind {
            ReminderKind::Eyes => self.enabled.eyes,
            ReminderKind::Stand => self.enabled.stand,
            ReminderKind::Water => self.enabled.water,
            ReminderKind::Neck => self.enabled.neck,
            ReminderKind::PomodoroBreak | ReminderKind::PomodoroFocus => self.enabled.pomodoro,
            ReminderKind::BigBreak => self.enabled.big_break,
            ReminderKind::Lunch => self.enabled.lunch,
            ReminderKind::OffWork => self.enabled.off_work,
            ReminderKind::Sleep => self.enabled.sleep,
        }
    }

    /// 周期型提醒的触发间隔（None 表示按时间点触发）
    pub fn interval_sec(&self, kind: ReminderKind) -> Option<u64> {
        match kind {
            ReminderKind::Eyes => Some(self.eyes_interval_sec),
            ReminderKind::Stand => Some(self.stand_interval_sec),
            ReminderKind::Water => Some(self.water_interval_sec),
            ReminderKind::Neck => Some(self.neck_interval_sec),
            ReminderKind::PomodoroBreak => Some(self.pomodoro_focus_sec),
            ReminderKind::PomodoroFocus => Some(self.pomodoro_break_sec),
            ReminderKind::BigBreak => Some(self.big_break_interval_sec),
            _ => None,
        }
    }
}

/// 获取数据目录路径
fn build_paths() -> Result<Paths> {
    let base = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("无法定位用户配置目录"))?
        .join(APP_DIR);
    std::fs::create_dir_all(&base).with_context(|| format!("创建配置目录失败: {:?}", base))?;
    Ok(Paths {
        config_file: base.join(CONFIG_FILE),
        db_file: base.join(DB_FILE),
        data_dir: base,
    })
}

/// 加载配置；若不存在则写默认配置
pub fn load_or_default() -> Result<Config> {
    let paths = build_paths()?;
    let cfg = if paths.config_file.exists() {
        let text = std::fs::read_to_string(&paths.config_file)
            .with_context(|| format!("读取配置失败: {:?}", paths.config_file))?;
        match toml::from_str::<Config>(&text) {
            Ok(mut c) => {
                c.paths = paths;
                // 为旧版本配置补写新增字段，避免用户需要手动打开设置页才能落盘默认排班。
                let schedule_missing = toml::from_str::<toml::Value>(&text)
                    .ok()
                    .and_then(|v| v.get("general").cloned())
                    .and_then(|v| v.as_table().cloned())
                    .map(|table| {
                        !table.contains_key("auto_schedule")
                            || !table.contains_key("work_start")
                            || !table.contains_key("work_end")
                    })
                    .unwrap_or(true);
                if schedule_missing {
                    save(&c)?;
                }
                c
            }
            Err(e) => {
                log::warn!("配置文件解析失败 ({e})，使用默认值并备份原文件");
                let backup = paths.config_file.with_extension("toml.bak");
                let _ = std::fs::rename(&paths.config_file, &backup);
                let mut c = Config::default();
                c.paths = paths;
                save(&c)?;
                c
            }
        }
    } else {
        let mut c = Config::default();
        c.paths = paths;
        save(&c)?;
        c
    };
    Ok(cfg)
}

/// 持久化配置
pub fn save(cfg: &Config) -> Result<()> {
    let text = toml::to_string_pretty(cfg).context("序列化配置失败")?;
    std::fs::write(&cfg.paths.config_file, text)
        .with_context(|| format!("写入配置失败: {:?}", cfg.paths.config_file))?;
    Ok(())
}

/// 解析 "HH:MM" 形式时间
pub fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_enables_nine_to_seven_schedule() {
        let cfg = Config::default();
        assert!(cfg.general.auto_schedule);
        assert_eq!(cfg.general.work_start, "09:00");
        assert_eq!(cfg.general.work_end, "19:00");
    }

    #[test]
    fn legacy_general_config_gets_schedule_defaults() {
        let cfg = Config::default();
        let text = toml::to_string(&cfg).unwrap();
        let mut value = toml::from_str::<toml::Value>(&text).unwrap();
        let general = value
            .get_mut("general")
            .and_then(toml::Value::as_table_mut)
            .unwrap();
        general.remove("auto_schedule");
        general.remove("work_start");
        general.remove("work_end");

        let loaded: Config = toml::from_str(&toml::to_string(&value).unwrap()).unwrap();
        assert!(loaded.general.auto_schedule);
        assert_eq!(loaded.general.work_start, "09:00");
        assert_eq!(loaded.general.work_end, "19:00");
    }
}
