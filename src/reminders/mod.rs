// 提醒类型定义 —— 所有提醒种类的元数据集中此处

use serde::{Deserialize, Serialize};
use std::fmt;

/// 提醒强度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intensity {
    /// 仅桌面通知
    Soft,
    /// 通知 + 声音
    Medium,
    /// 全屏模态遮罩 + 声音
    Strong,
}

/// 所有提醒种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReminderKind {
    /// 20-20-20 护眼
    Eyes,
    /// 起身微休息
    Stand,
    /// 喝水
    Water,
    /// 颈椎活动
    Neck,
    /// 番茄钟工作结束（进入小休息）
    PomodoroBreak,
    /// 番茄钟休息结束（回到专注）
    PomodoroFocus,
    /// 大休息（强制全屏）
    BigBreak,
    /// 午餐提醒
    Lunch,
    /// 下班建议
    OffWork,
    /// 睡眠提醒
    Sleep,
}

impl ReminderKind {
    /// 提醒在 UI / 通知中显示的名字
    pub fn label(self) -> &'static str {
        match self {
            ReminderKind::Eyes => "护眼休息",
            ReminderKind::Stand => "起身活动",
            ReminderKind::Water => "喝水提醒",
            ReminderKind::Neck => "颈椎活动",
            ReminderKind::PomodoroBreak => "番茄钟休息",
            ReminderKind::PomodoroFocus => "回到专注",
            ReminderKind::BigBreak => "大休息（强制）",
            ReminderKind::Lunch => "午餐时间",
            ReminderKind::OffWork => "建议下班",
            ReminderKind::Sleep => "准备睡眠",
        }
    }

    /// 简介，作为通知正文 fallback
    pub fn brief(self) -> &'static str {
        match self {
            ReminderKind::Eyes => "看向 6 米外的物体 20 秒，让眼睛放松一下",
            ReminderKind::Stand => "站起来活动一下，舒展身体",
            ReminderKind::Water => "起身喝一杯水（约 250 ml）",
            ReminderKind::Neck => "做一组颈椎活动操，缓解僵硬",
            ReminderKind::PomodoroBreak => "番茄钟完成，进入 5 分钟小休息",
            ReminderKind::PomodoroFocus => "休息结束，回到专注工作",
            ReminderKind::BigBreak => "已连续工作较久，请离开座位休息 5 分钟",
            ReminderKind::Lunch => "午餐时间到啦，记得补充能量",
            ReminderKind::OffWork => "今天已工作满 8 小时，建议收工休息",
            ReminderKind::Sleep => "准备进入睡眠时间，关上屏幕吧",
        }
    }

    /// 默认强度
    pub fn intensity(self) -> Intensity {
        match self {
            ReminderKind::Eyes | ReminderKind::Stand | ReminderKind::Water => Intensity::Soft,
            ReminderKind::Neck
            | ReminderKind::PomodoroBreak
            | ReminderKind::PomodoroFocus
            | ReminderKind::Lunch
            | ReminderKind::Sleep
            | ReminderKind::OffWork => Intensity::Medium,
            ReminderKind::BigBreak => Intensity::Strong,
        }
    }

    /// 对应健康知识库类目（用于随机抽取 tip）
    pub fn tip_category(self) -> Option<&'static str> {
        match self {
            ReminderKind::Eyes => Some("eyes"),
            ReminderKind::Stand => Some("legs"),
            ReminderKind::Water => Some("nutrition"),
            ReminderKind::Neck => Some("neck"),
            ReminderKind::PomodoroBreak | ReminderKind::BigBreak => Some("breathing"),
            ReminderKind::PomodoroFocus | ReminderKind::Lunch => None,
            ReminderKind::OffWork | ReminderKind::Sleep => Some("sleep"),
        }
    }

    /// 持久化用字符串 key（写入 SQLite）
    pub fn db_key(self) -> &'static str {
        match self {
            ReminderKind::Eyes => "eyes",
            ReminderKind::Stand => "stand",
            ReminderKind::Water => "water",
            ReminderKind::Neck => "neck",
            ReminderKind::PomodoroBreak => "pomodoro_break",
            ReminderKind::PomodoroFocus => "pomodoro_focus",
            ReminderKind::BigBreak => "big_break",
            ReminderKind::Lunch => "lunch",
            ReminderKind::OffWork => "off_work",
            ReminderKind::Sleep => "sleep",
        }
    }

    /// 所有种类的迭代器
    pub fn all() -> &'static [ReminderKind] {
        const ALL: [ReminderKind; 10] = [
            ReminderKind::Eyes,
            ReminderKind::Stand,
            ReminderKind::Water,
            ReminderKind::Neck,
            ReminderKind::PomodoroBreak,
            ReminderKind::PomodoroFocus,
            ReminderKind::BigBreak,
            ReminderKind::Lunch,
            ReminderKind::OffWork,
            ReminderKind::Sleep,
        ];
        &ALL
    }
}

impl fmt::Display for ReminderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
