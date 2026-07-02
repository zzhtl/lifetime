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
    Breathing,
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
            PracticeCategory::Breathing => "breathing",
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
            PracticeCategory::Breathing => "呼吸法门",
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
            PracticeCategory::Breathing => "☁",
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
            PracticeCategory::Breathing => (0x7f, 0xb4, 0xd8),
            PracticeCategory::XianCultivation => (0xd0, 0xc0, 0x72),
        }
    }

    pub fn all() -> &'static [PracticeCategory] {
        const ALL: [PracticeCategory; 10] = [
            PracticeCategory::Diet,
            PracticeCategory::WalkingRunning,
            PracticeCategory::TaijiQigong,
            PracticeCategory::Stretching,
            PracticeCategory::YijinJing,
            PracticeCategory::Prevention,
            PracticeCategory::Immunity,
            PracticeCategory::MindBreath,
            PracticeCategory::Breathing,
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

// ── 修为境界：由「修炼打卡」累计数驱动的成长阶梯 ──
// 注意：这与 PracticeStage（功法难度标签：入门/筑基/进阶/长期）是两个不同维度，勿混淆。
// 境界由累计修为值（打卡数）纯函数算出，不额外落库存状态。

/// 修为境界阶梯：(达到该境界所需累计修为值, 境界名)，按门槛升序。
const REALMS: [(i64, &str); 8] = [
    (0, "凡尘"),
    (5, "炼气"),
    (15, "筑基"),
    (35, "金丹"),
    (70, "元婴"),
    (120, "化神"),
    (200, "炼虚"),
    (320, "合道"),
];

/// 当前修为进度快照，供 UI 渲染境界横幅与进度条。
#[derive(Debug, Clone, PartialEq)]
pub struct RealmProgress {
    /// 当前境界名
    pub name: &'static str,
    /// 下一境界名；已达顶境则为 None
    pub next_name: Option<&'static str>,
    /// 累计修为值（打卡数）
    pub points: i64,
    /// 从当前境界到下一境界还需多少次（已达顶为 0）
    pub need: i64,
    /// 当前境界内的进度比例 0.0~1.0（已达顶为 1.0）
    pub ratio: f32,
}

/// 由累计修为值（打卡数）计算当前境界与进度。纯函数。
pub fn realm_progress(points: i64) -> RealmProgress {
    let p = points.max(0);
    // 当前境界 = 最后一个门槛 <= p 的那一档（REALMS 按门槛升序）
    let mut idx = 0usize;
    for (i, (threshold, _)) in REALMS.iter().enumerate() {
        if p >= *threshold {
            idx = i;
        } else {
            break;
        }
    }
    let (cur_threshold, name) = REALMS[idx];
    if idx + 1 < REALMS.len() {
        let (next_threshold, next_name) = REALMS[idx + 1];
        let span = (next_threshold - cur_threshold).max(1);
        let into = p - cur_threshold;
        RealmProgress {
            name,
            next_name: Some(next_name),
            points: p,
            need: next_threshold - p,
            ratio: (into as f32 / span as f32).clamp(0.0, 1.0),
        }
    } else {
        RealmProgress {
            name,
            next_name: None,
            points: p,
            need: 0,
            ratio: 1.0,
        }
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
    fn breathing_includes_modern_evidence_based_methods() {
        let lib = PracticeLibrary::load().unwrap();
        let titles: Vec<&str> = lib
            .by_category(PracticeCategory::Breathing)
            .into_iter()
            .map(|p| p.title.as_str())
            .collect();
        // 顶层「呼吸法门」练习台预设对应的现代法门，不可缺失
        for expected in ["4-7-8", "箱式", "生理叹息", "共振", "腹式", "延长呼气"] {
            assert!(
                titles.iter().any(|title| title.contains(expected)),
                "缺少现代呼吸法门: {expected}"
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

    #[test]
    fn realm_progress_maps_points_to_realms() {
        assert_eq!(realm_progress(0).name, "凡尘");
        assert_eq!(realm_progress(4).name, "凡尘");
        assert_eq!(realm_progress(5).name, "炼气"); // 门槛边界
        assert_eq!(realm_progress(15).name, "筑基");

        let p = realm_progress(10); // 炼气(5)→筑基(15) 正中
        assert_eq!(p.name, "炼气");
        assert_eq!(p.next_name, Some("筑基"));
        assert_eq!(p.need, 5);
        assert!((p.ratio - 0.5).abs() < 1e-6);

        let top = realm_progress(1000); // 达顶境
        assert_eq!(top.name, "合道");
        assert_eq!(top.next_name, None);
        assert_eq!(top.need, 0);
        assert_eq!(top.ratio, 1.0);
    }
}
