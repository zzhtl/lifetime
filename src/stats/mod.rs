// 统计聚合
// 把 db 层的原始查询整理成 UI 友好的视图数据

use crate::db::{kind_distribution, recent_days, Db, DailyPoint, DailySummary};
use crate::reminders::ReminderKind;
use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub struct StatsView {
    pub today: DailySummary,
    pub last_30: Vec<DailyPoint>,
    pub kind_dist_30d: Vec<(String, i64)>,
    /// 连续达标天数
    pub streak: i64,
    /// 今日大休息跟练 (已完成, 已提示总数)
    pub big_break_today: (i64, i64),
}

impl StatsView {
    pub fn load(db: &Db) -> Result<Self> {
        Ok(Self {
            today: crate::db::get_today(db)?,
            last_30: recent_days(db, 30)?,
            kind_dist_30d: kind_distribution(db, 30)?,
            streak: crate::db::streak(db)?,
            big_break_today: crate::db::today_big_break_completion(db)?,
        })
    }
}

/// 将 db_key 映射回中文 label（用于柱状图标签）
pub fn kind_label(db_key: &str) -> &'static str {
    for k in ReminderKind::all() {
        if k.db_key() == db_key {
            return k.label();
        }
    }
    "未知"
}

/// 把秒数格式化为 H:MM:SS
pub fn fmt_hms(secs: i64) -> String {
    let s = secs.max(0);
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let s = s % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
