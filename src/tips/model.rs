// 单条 tip 数据结构

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipCategory {
    Eyes,
    Neck,
    Back,
    Wrist,
    Legs,
    Breathing,
    Nutrition,
    Posture,
    Sleep,
}

impl TipCategory {
    pub fn key(&self) -> &'static str {
        match self {
            TipCategory::Eyes => "eyes",
            TipCategory::Neck => "neck",
            TipCategory::Back => "back",
            TipCategory::Wrist => "wrist",
            TipCategory::Legs => "legs",
            TipCategory::Breathing => "breathing",
            TipCategory::Nutrition => "nutrition",
            TipCategory::Posture => "posture",
            TipCategory::Sleep => "sleep",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TipCategory::Eyes => "护眼",
            TipCategory::Neck => "颈椎与肩",
            TipCategory::Back => "腰背",
            TipCategory::Wrist => "手腕（防 RSI）",
            TipCategory::Legs => "腿部循环",
            TipCategory::Breathing => "呼吸与心理",
            TipCategory::Nutrition => "饮食与水分",
            TipCategory::Posture => "姿势与工位",
            TipCategory::Sleep => "睡眠",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TipCategory::Eyes => "👁",
            TipCategory::Neck => "🦴",
            TipCategory::Back => "🧘",
            TipCategory::Wrist => "✋",
            TipCategory::Legs => "🦵",
            TipCategory::Breathing => "🌬",
            TipCategory::Nutrition => "💧",
            TipCategory::Posture => "🪑",
            TipCategory::Sleep => "🌙",
        }
    }

    /// 分类主题色（RGB），UI 用作标签/卡片点缀。保持 tips 模块不依赖 egui。
    pub fn accent(&self) -> (u8, u8, u8) {
        match self {
            TipCategory::Eyes => (0x5c, 0xb8, 0xe0),       // 蓝
            TipCategory::Neck => (0xe0, 0x8a, 0x6a),       // 砖红
            TipCategory::Back => (0x8f, 0xb8, 0x6a),       // 草绿
            TipCategory::Wrist => (0xd8, 0xa6, 0x57),      // 琥珀
            TipCategory::Legs => (0x6a, 0xc2, 0x9a),       // 青绿
            TipCategory::Breathing => (0xb6, 0x8f, 0xe0),  // 紫
            TipCategory::Nutrition => (0x5c, 0xc2, 0xc2),  // 水蓝
            TipCategory::Posture => (0xc2, 0x96, 0x7a),    // 棕
            TipCategory::Sleep => (0x8a, 0x9a, 0xd8),      // 靛
        }
    }

    pub fn all() -> &'static [TipCategory] {
        const ALL: [TipCategory; 9] = [
            TipCategory::Eyes,
            TipCategory::Neck,
            TipCategory::Back,
            TipCategory::Wrist,
            TipCategory::Legs,
            TipCategory::Breathing,
            TipCategory::Nutrition,
            TipCategory::Posture,
            TipCategory::Sleep,
        ];
        &ALL
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tip {
    pub category: TipCategory,
    pub title: String,

    /// 推荐时长，秒；0 表示不限
    #[serde(default)]
    pub duration_secs: u32,

    /// 推荐频率（如"每小时一次"、"每天 3-5 次"、"每次工作日"）
    #[serde(default)]
    pub frequency: String,

    /// 是否适合作为办公室内休息跟练 / 通知动作
    #[serde(default)]
    pub office_break: bool,

    /// 动作步骤
    pub steps: Vec<String>,

    /// 主要益处（一句话）
    pub benefit: String,

    /// 科学依据 / 机理简述
    #[serde(default)]
    pub science: String,

    /// 注意事项、禁忌人群
    #[serde(default)]
    pub caution: String,

    /// 进阶变式 / 加强版动作
    #[serde(default)]
    pub variants: Vec<String>,
}

/// 大休息「分段跟练」中的一个小节：一条动作 + 该节分配到的时长
#[derive(Debug, Clone)]
pub struct RoutineSegment {
    pub category: TipCategory,
    pub title: String,
    pub steps: Vec<String>,
    pub benefit: String,
    /// 本小节倒计时秒数
    pub seconds: u64,
}
