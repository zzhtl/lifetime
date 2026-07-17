// UI 模块
// 顶层路由：主页 / 呼吸 / 知识库 / 修炼 / 统计 / 设置

pub mod break_window;
pub mod breathing;
pub mod dashboard;
pub mod fonts;
pub mod library;
pub mod practice;
pub mod settings;
pub mod stats_view;
pub mod theme;
pub mod widgets;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    Dashboard,
    Breathing,
    Library,
    Practice,
    Stats,
    Settings,
}

impl Default for View {
    fn default() -> Self {
        View::Dashboard
    }
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Dashboard => "主页",
            View::Breathing => "呼吸法门",
            View::Library => "健康知识",
            View::Practice => "养生修炼",
            View::Stats => "统计",
            View::Settings => "设置",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            View::Dashboard => "⌂",
            View::Breathing => "◌",
            View::Library => "▤",
            View::Practice => "◇",
            View::Stats => "▥",
            View::Settings => "⚙",
        }
    }

    pub fn all() -> &'static [View] {
        const ALL: [View; 6] = [
            View::Dashboard,
            View::Breathing,
            View::Library,
            View::Practice,
            View::Stats,
            View::Settings,
        ];
        &ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_navigation_has_six_unique_views() {
        let all = View::all();
        assert_eq!(all.len(), 6);
        for (index, view) in all.iter().enumerate() {
            assert!(!view.label().is_empty());
            assert!(!view.icon().is_empty());
            assert!(!all[..index].contains(view), "一级导航存在重复项: {view:?}");
        }
    }
}
