// 健康知识库
// TOML 数据嵌入二进制，启动时一次解析

mod model;

pub use model::{RoutineSegment, Tip, TipCategory};

use anyhow::{Context, Result};
use rand::seq::{IteratorRandom, SliceRandom};
use serde::Deserialize;

const TIPS_DATA: &str = include_str!("../../data/tips.toml");

#[derive(Deserialize)]
struct RawTips {
    tip: Vec<Tip>,
}

#[derive(Debug, Clone, Default)]
pub struct Library {
    tips: Vec<Tip>,
}

impl Library {
    pub fn load() -> Result<Self> {
        let raw: RawTips = toml::from_str(TIPS_DATA).context("解析 tips.toml 失败")?;
        Ok(Self { tips: raw.tip })
    }

    #[cfg(test)]
    pub fn all(&self) -> &[Tip] {
        &self.tips
    }

    pub fn by_category(&self, cat: TipCategory) -> Vec<&Tip> {
        self.tips.iter().filter(|t| t.category == cat).collect()
    }

    pub fn by_category_key(&self, key: &str) -> Vec<&Tip> {
        self.tips
            .iter()
            .filter(|t| t.category.key() == key)
            .collect()
    }

    // 单条随机抽取：大休息改为分段跟练后，生产路径改用 build_break_routine；保留供测试与将来复用
    #[allow(dead_code)]
    pub fn random_for_category(&self, key: &str) -> Option<&Tip> {
        let mut rng = rand::thread_rng();
        let list = self.by_category_key(key);
        list.choose(&mut rng).copied()
    }

    /// 生成大休息「分段跟练」路线：按总时长拆成 3-5 个小节，
    /// 每节取自不同的身体部位类目（轮换打乱顺序），并尽量避开 avoid 中的标题
    /// （上一次用过的动作），从而每次组合不同、不重复。
    pub fn build_break_routine(&self, total_secs: u64, avoid: &[String]) -> Vec<RoutineSegment> {
        let total = total_secs.max(1);
        let mut rng = rand::thread_rng();

        // 适合工间起身的多部位轮换池；打乱保证每次顺序/组合不同
        let mut pool = [
            TipCategory::Neck,
            TipCategory::Back,
            TipCategory::Legs,
            TipCategory::Eyes,
            TipCategory::Breathing,
            TipCategory::Wrist,
            TipCategory::Posture,
        ];
        pool.shuffle(&mut rng);

        // 约每 75s 一节，夹在 [3,5]，并只取有内容的类目
        let want = (total / 75).clamp(3, 5) as usize;
        let cats: Vec<TipCategory> = pool
            .into_iter()
            .filter(|c| !self.by_category(*c).is_empty())
            .take(want)
            .collect();
        let n = cats.len();
        if n == 0 {
            return Vec::new();
        }

        // 时长均分，余数并入最后一节
        let base = total / n as u64;
        let mut segments = Vec::with_capacity(n);
        for (i, cat) in cats.iter().enumerate() {
            let seconds = if i == n - 1 {
                total - base * (n as u64 - 1)
            } else {
                base
            };
            let list = self.by_category(*cat);
            // 优先在"未被 avoid"的动作里随机；都被避开则退回全集随机
            let chosen: Option<&Tip> = list
                .iter()
                .filter(|t| !avoid.contains(&t.title))
                .choose(&mut rng)
                .copied()
                .or_else(|| list.choose(&mut rng).copied());
            if let Some(t) = chosen {
                segments.push(RoutineSegment {
                    category: *cat,
                    title: t.title.clone(),
                    steps: t.steps.clone(),
                    benefit: t.benefit.clone(),
                    seconds,
                });
            }
        }
        segments
    }

    #[cfg(test)]
    pub fn categories(&self) -> Vec<TipCategory> {
        let mut seen = Vec::new();
        for t in &self.tips {
            if !seen.contains(&t.category) {
                seen.push(t.category);
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_loads_and_has_all_categories() {
        let lib = Library::load().expect("tips.toml 应能解析");
        assert!(lib.all().len() >= 30, "至少 30 条 tip");
        let cats = lib.categories();
        // 9 大类必须齐全
        for c in TipCategory::all() {
            assert!(cats.contains(c), "缺少类目 {c:?}");
        }
    }

    #[test]
    fn random_returns_some_for_each_category() {
        let lib = Library::load().unwrap();
        for c in TipCategory::all() {
            assert!(lib.random_for_category(c.key()).is_some(), "类目 {c:?} 没有 tip");
        }
    }

    #[test]
    fn routine_has_expected_segments_and_total() {
        let lib = Library::load().unwrap();
        let total = 300;
        let r = lib.build_break_routine(total, &[]);
        assert!(r.len() >= 3 && r.len() <= 5, "节数应在 3-5 之间，实际 {}", r.len());
        assert_eq!(
            r.iter().map(|s| s.seconds).sum::<u64>(),
            total,
            "各节时长应累加等于总时长"
        );
        // 同一路线内类目不重复
        let mut keys: Vec<&str> = r.iter().map(|s| s.category.key()).collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n, "同一路线类目应不重复");
    }

    #[test]
    fn routine_avoids_previous_titles() {
        let lib = Library::load().unwrap();
        let first = lib.build_break_routine(300, &[]);
        let avoid: Vec<String> = first.iter().map(|s| s.title.clone()).collect();
        let second = lib.build_break_routine(300, &avoid);
        for s in &second {
            assert!(!avoid.contains(&s.title), "应避开上次用过的动作: {}", s.title);
        }
    }
}
