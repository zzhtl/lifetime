// 健康知识库
// TOML 数据嵌入二进制，启动时一次解析

mod model;

pub use model::{Tip, TipCategory};

use anyhow::{Context, Result};
use rand::seq::SliceRandom;
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

    pub fn random_for_category(&self, key: &str) -> Option<&Tip> {
        let mut rng = rand::thread_rng();
        let list = self.by_category_key(key);
        list.choose(&mut rng).copied()
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
}
