// 养生修炼体系
// TOML 数据嵌入二进制，提供体系化的长期健康方案。

use anyhow::{Context, Result};
use serde::Deserialize;

const PRACTICES_DATA: &str = include_str!("../../data/practices.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeCategory {
    Diet,
    WalkingRunning,
    TaijiQigong,
    Stretching,
    YijinJing,
    Prevention,
    Immunity,
    MindBreath,
    XianCultivation,
}

impl PracticeCategory {
    pub fn key(&self) -> &'static str {
        match self {
            PracticeCategory::Diet => "diet",
            PracticeCategory::WalkingRunning => "walking_running",
            PracticeCategory::TaijiQigong => "taiji_qigong",
            PracticeCategory::Stretching => "stretching",
            PracticeCategory::YijinJing => "yijin_jing",
            PracticeCategory::Prevention => "prevention",
            PracticeCategory::Immunity => "immunity",
            PracticeCategory::MindBreath => "mind_breath",
            PracticeCategory::XianCultivation => "xian_cultivation",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PracticeCategory::Diet => "饮食有节",
            PracticeCategory::WalkingRunning => "起居有常",
            PracticeCategory::TaijiQigong => "导引按跷",
            PracticeCategory::Stretching => "不妄作劳",
            PracticeCategory::YijinJing => "四时调神",
            PracticeCategory::Prevention => "治未病",
            PracticeCategory::Immunity => "正气固本",
            PracticeCategory::MindBreath => "恬淡虚无",
            PracticeCategory::XianCultivation => "修仙次第",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PracticeCategory::Diet => "🍚",
            PracticeCategory::WalkingRunning => "🌅",
            PracticeCategory::TaijiQigong => "☯",
            PracticeCategory::Stretching => "⚖",
            PracticeCategory::YijinJing => "🌿",
            PracticeCategory::Prevention => "🛡",
            PracticeCategory::Immunity => "🔥",
            PracticeCategory::MindBreath => "🌬",
            PracticeCategory::XianCultivation => "⛰",
        }
    }

    pub fn accent(&self) -> (u8, u8, u8) {
        match self {
            PracticeCategory::Diet => (0x79, 0xc2, 0x8a),
            PracticeCategory::WalkingRunning => (0x68, 0xb8, 0xd8),
            PracticeCategory::TaijiQigong => (0xd0, 0xa5, 0x5f),
            PracticeCategory::Stretching => (0xb7, 0x8a, 0xd8),
            PracticeCategory::YijinJing => (0xd8, 0x8f, 0x77),
            PracticeCategory::Prevention => (0x7f, 0xb1, 0xe0),
            PracticeCategory::Immunity => (0x8f, 0xc4, 0x68),
            PracticeCategory::MindBreath => (0x6f, 0xc2, 0xb8),
            PracticeCategory::XianCultivation => (0xd0, 0xc0, 0x72),
        }
    }

    pub fn all() -> &'static [PracticeCategory] {
        const ALL: [PracticeCategory; 9] = [
            PracticeCategory::Diet,
            PracticeCategory::WalkingRunning,
            PracticeCategory::TaijiQigong,
            PracticeCategory::Stretching,
            PracticeCategory::YijinJing,
            PracticeCategory::Prevention,
            PracticeCategory::Immunity,
            PracticeCategory::MindBreath,
            PracticeCategory::XianCultivation,
        ];
        &ALL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeStage {
    Entry,
    Foundation,
    Advanced,
    LongTerm,
}

impl PracticeStage {
    pub fn label(&self) -> &'static str {
        match self {
            PracticeStage::Entry => "入门",
            PracticeStage::Foundation => "筑基",
            PracticeStage::Advanced => "进阶",
            PracticeStage::LongTerm => "长期",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeScene {
    Office,
    Home,
    Outdoor,
    Bedtime,
}

impl PracticeScene {
    pub fn label(&self) -> &'static str {
        match self {
            PracticeScene::Office => "办公室",
            PracticeScene::Home => "居家",
            PracticeScene::Outdoor => "户外",
            PracticeScene::Bedtime => "睡前",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    Classic,
    Guideline,
    Government,
    MedicalCenter,
    TraditionalAdapted,
}

impl EvidenceLevel {
    pub fn label(&self) -> &'static str {
        match self {
            EvidenceLevel::Classic => "经典原典",
            EvidenceLevel::Guideline => "指南",
            EvidenceLevel::Government => "政府/公共卫生",
            EvidenceLevel::MedicalCenter => "医学中心",
            EvidenceLevel::TraditionalAdapted => "传统改编",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PracticeSource {
    pub name: String,
    pub url: String,
    pub level: EvidenceLevel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Practice {
    pub category: PracticeCategory,
    pub title: String,
    pub stage: PracticeStage,
    #[serde(default)]
    pub duration_mins: u32,
    #[serde(default)]
    pub frequency: String,
    #[serde(default)]
    pub scenes: Vec<PracticeScene>,
    pub summary: String,
    pub steps: Vec<String>,
    pub benefit: String,
    pub science: String,
    pub caution: String,
    pub sources: Vec<PracticeSource>,
}

#[derive(Deserialize)]
struct RawPractices {
    practice: Vec<Practice>,
}

#[derive(Debug, Clone, Default)]
pub struct PracticeLibrary {
    practices: Vec<Practice>,
}

impl PracticeLibrary {
    pub fn load() -> Result<Self> {
        let raw: RawPractices =
            toml::from_str(PRACTICES_DATA).context("解析 practices.toml 失败")?;
        Ok(Self {
            practices: raw.practice,
        })
    }

    pub fn all(&self) -> &[Practice] {
        &self.practices
    }

    pub fn by_category(&self, cat: PracticeCategory) -> Vec<&Practice> {
        self.practices.iter().filter(|p| p.category == cat).collect()
    }

    pub fn categories(&self) -> Vec<PracticeCategory> {
        let mut seen = Vec::new();
        for p in &self.practices {
            if !seen.contains(&p.category) {
                seen.push(p.category);
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn practices_load_and_cover_all_categories() {
        let lib = PracticeLibrary::load().expect("practices.toml 应能解析");
        assert!(lib.all().len() >= 35, "修炼体系至少 35 条");
        let cats = lib.categories();
        for c in PracticeCategory::all() {
            assert!(cats.contains(c), "缺少修炼分类 {c:?}");
            assert!(!lib.by_category(*c).is_empty(), "分类 {c:?} 不应为空");
        }
    }

    #[test]
    fn practices_have_required_safety_and_sources() {
        let lib = PracticeLibrary::load().unwrap();
        for p in lib.all() {
            assert!(!p.title.trim().is_empty(), "标题不能为空");
            assert!(!p.summary.trim().is_empty(), "{} 缺少摘要", p.title);
            assert!(!p.steps.is_empty(), "{} 缺少步骤", p.title);
            assert!(!p.benefit.trim().is_empty(), "{} 缺少益处", p.title);
            assert!(!p.science.trim().is_empty(), "{} 缺少现代解释", p.title);
            assert!(!p.caution.trim().is_empty(), "{} 缺少注意事项", p.title);
            assert!(!p.sources.is_empty(), "{} 缺少来源", p.title);
            for source in &p.sources {
                assert!(!source.name.trim().is_empty(), "{} 来源名为空", p.title);
                assert!(source.url.starts_with("https://"), "{} 来源 URL 非 https", p.title);
            }
        }
    }

    #[test]
    fn medical_claims_avoid_absolute_promises() {
        let lib = PracticeLibrary::load().unwrap();
        let banned = ["包治", "根治", "保证长生", "永生", "百病不生", "药到病除"];
        for p in lib.all() {
            let text = format!(
                "{}{}{}{}{}",
                p.title, p.summary, p.benefit, p.science, p.caution
            );
            for word in banned {
                assert!(!text.contains(word), "{} 含有绝对疗效词: {word}", p.title);
            }
        }
    }

    #[test]
    fn qi_training_methods_are_present() {
        let lib = PracticeLibrary::load().unwrap();
        let titles: Vec<&str> = lib
            .by_category(PracticeCategory::MindBreath)
            .into_iter()
            .map(|p| p.title.as_str())
            .collect();
        for expected in ["打坐守息", "内观身受", "丹田息", "周天观想", "行禅步息"] {
            assert!(
                titles.iter().any(|title| title.contains(expected)),
                "缺少练气法门: {expected}"
            );
        }
    }

    #[test]
    fn xian_cultivation_sequence_is_present_and_safe() {
        let lib = PracticeLibrary::load().unwrap();
        let methods = lib.by_category(PracticeCategory::XianCultivation);
        assert!(methods.len() >= 17, "修仙次第至少 17 法");

        let titles: Vec<&str> = methods.iter().map(|p| p.title.as_str()).collect();
        for expected in [
            "修仙总纲",
            "百日筑基",
            "坐忘七阶",
            "清静内观",
            "太乙金华宗旨",
            "黄庭经",
            "阴符经",
            "性命圭旨",
            "钟吕传道集",
            "入药镜",
            "辨伪避坑",
        ] {
            assert!(
                titles.iter().any(|title| title.contains(expected)),
                "缺少修仙条目: {expected}"
            );
        }

        let safety_text = methods
            .iter()
            .map(|p| format!("{}{}{}{}", p.title, p.summary, p.steps.join(""), p.caution))
            .collect::<Vec<String>>()
            .join("\n");
        for expected in ["不承诺超自然结果", "严禁长时间闭息", "不自行辟谷断食"] {
            assert!(safety_text.contains(expected), "缺少安全红线: {expected}");
        }
    }
}
