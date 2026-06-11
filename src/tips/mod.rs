// 健康知识库
// TOML 数据嵌入二进制，启动时一次解析

mod model;

pub use model::{RoutineSegment, Tip, TipCategory};

use anyhow::{Context, Result};
use chrono::{Datelike, Local, Weekday};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipMode {
    Office,
    Wellness,
}

impl TipMode {
    pub fn today() -> Self {
        Self::from_weekday(Local::now().weekday())
    }

    pub fn from_weekday(day: Weekday) -> Self {
        match day {
            Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri => {
                Self::Office
            }
            Weekday::Sat | Weekday::Sun => Self::Wellness,
        }
    }
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

    pub fn office_break_by_category(&self, cat: TipCategory) -> Vec<&Tip> {
        self.tips
            .iter()
            .filter(|t| t.category == cat && t.office_break)
            .collect()
    }

    pub fn office_break_by_category_key(&self, key: &str) -> Vec<&Tip> {
        self.tips
            .iter()
            .filter(|t| t.category.key() == key && t.office_break)
            .collect()
    }

    pub fn by_category_for_mode(&self, cat: TipCategory, mode: TipMode) -> Vec<&Tip> {
        match mode {
            TipMode::Office => self.office_break_by_category(cat),
            TipMode::Wellness => self.by_category(cat),
        }
    }

    pub fn by_category_key_for_mode(&self, key: &str, mode: TipMode) -> Vec<&Tip> {
        match mode {
            TipMode::Office => self.office_break_by_category_key(key),
            TipMode::Wellness => self.by_category_key(key),
        }
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
        self.build_break_routine_for_mode(total_secs, avoid, TipMode::today())
    }

    pub fn build_break_routine_for_mode(
        &self,
        total_secs: u64,
        avoid: &[String],
        mode: TipMode,
    ) -> Vec<RoutineSegment> {
        let total = total_secs.max(1);
        let mut rng = rand::thread_rng();

        // 多部位轮换池；工作日只取办公室动作，周末允许更完整的养生运动。
        let mut pool = [
            TipCategory::Neck,
            TipCategory::Back,
            TipCategory::Legs,
            TipCategory::Eyes,
            TipCategory::Breathing,
            TipCategory::Wrist,
        ];
        pool.shuffle(&mut rng);

        // 约每 75s 一节，夹在 [3,5]，并只取有内容的类目
        let want = (total / 75).clamp(3, 5) as usize;
        let cats: Vec<TipCategory> = pool
            .into_iter()
            .filter(|c| !self.by_category_for_mode(*c, mode).is_empty())
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
            let list = self.by_category_for_mode(*cat, mode);
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
    use chrono::Weekday;

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
    fn tip_mode_follows_weekday() {
        assert_eq!(TipMode::from_weekday(Weekday::Mon), TipMode::Office);
        assert_eq!(TipMode::from_weekday(Weekday::Fri), TipMode::Office);
        assert_eq!(TipMode::from_weekday(Weekday::Sat), TipMode::Wellness);
        assert_eq!(TipMode::from_weekday(Weekday::Sun), TipMode::Wellness);
    }

    #[test]
    fn mode_filters_office_and_wellness_pools() {
        let lib = Library::load().unwrap();

        let office_legs = lib.by_category_for_mode(TipCategory::Legs, TipMode::Office);
        assert!(!office_legs.is_empty(), "工作日腿部动作池不应为空");
        assert!(
            office_legs.iter().all(|t| t.office_break),
            "工作日动作池只能包含办公室动作"
        );

        let wellness_legs = lib.by_category_for_mode(TipCategory::Legs, TipMode::Wellness);
        assert!(
            wellness_legs.iter().any(|t| !t.office_break),
            "周末动作池应包含更完整的非办公室运动"
        );
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
        let r = lib.build_break_routine_for_mode(total, &[], TipMode::Office);
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
        let first = lib.build_break_routine_for_mode(300, &[], TipMode::Wellness);
        let avoid: Vec<String> = first.iter().map(|s| s.title.clone()).collect();
        let second = lib.build_break_routine_for_mode(300, &avoid, TipMode::Wellness);
        for s in &second {
            assert!(!avoid.contains(&s.title), "应避开上次用过的动作: {}", s.title);
        }
    }

    #[test]
    fn routine_uses_only_office_break_tips() {
        let lib = Library::load().unwrap();
        let routine = lib.build_break_routine_for_mode(300, &[], TipMode::Office);
        assert!(!routine.is_empty(), "办公室跟练路线不应为空");
        for segment in &routine {
            let tip = lib
                .all()
                .iter()
                .find(|t| t.title == segment.title)
                .expect("路线中的动作应来自知识库");
            assert!(
                tip.office_break,
                "大休息不应抽到非办公室动作: {}",
                tip.title
            );
        }
    }

    #[test]
    fn wellness_routine_can_use_non_office_tips() {
        let lib = Library::load().unwrap();
        let avoid: Vec<String> = lib
            .all()
            .iter()
            .filter(|t| t.office_break)
            .map(|t| t.title.clone())
            .collect();
        let routine = lib.build_break_routine_for_mode(300, &avoid, TipMode::Wellness);
        assert!(!routine.is_empty(), "周末跟练路线不应为空");
        let all_non_office = routine.iter().all(|segment| {
            lib.all()
                .iter()
                .find(|t| t.title == segment.title)
                .is_some_and(|t| !t.office_break)
        });
        assert!(all_non_office, "周末路线应允许非办公室动作");
    }
}
