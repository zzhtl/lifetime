// UI 模块
// 顶层路由：Dashboard / 知识库 / 统计 / 设置 / 关于

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
    About,
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
            View::About => "关于",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            View::Dashboard => "📊",
            View::Breathing => "🌬",
            View::Library => "📚",
            View::Practice => "☯",
            View::Stats => "📈",
            View::Settings => "⚙",
            View::About => "ℹ",
        }
    }

    pub fn all() -> &'static [View] {
        const ALL: [View; 7] = [
            View::Dashboard,
            View::Breathing,
            View::Library,
            View::Practice,
            View::Stats,
            View::Settings,
            View::About,
        ];
        &ALL
    }
}
