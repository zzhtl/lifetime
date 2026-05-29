// 业务查询封装

use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};

use super::Db;
use crate::reminders::ReminderKind;

/// 提醒处理结果
#[allow(dead_code)] // Skipped / Snoozed 是 API 完整性所需，将来在模态窗"暂缓"按钮接通
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderAction {
    Completed,
    Skipped,
    Snoozed,
}

impl ReminderAction {
    pub fn as_str(self) -> &'static str {
        match self {
            ReminderAction::Completed => "completed",
            ReminderAction::Skipped => "skipped",
            ReminderAction::Snoozed => "snoozed",
        }
    }
}

/// 当日汇总
#[derive(Debug, Clone, Default)]
pub struct DailySummary {
    pub date: String,
    pub work_seconds: i64,
    pub water_count: i64,
    pub stand_count: i64,
    pub eye_break_count: i64,
    pub neck_count: i64,
    pub pomodoros: i64,
    pub big_breaks: i64,
}

/// 历史一天的概要（用于趋势图）
#[derive(Debug, Clone)]
#[allow(dead_code)] // date / completed_events 留作未来 hover tooltip 使用
pub struct DailyPoint {
    pub date: NaiveDate,
    pub work_seconds: i64,
    pub completed_events: i64,
}

pub fn start_session(db: &Db) -> Result<i64> {
    let now = Utc::now().timestamp();
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO work_session (started_at) VALUES (?1)",
        params![now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn end_session(db: &Db, session_id: i64, total_secs: i64) -> Result<()> {
    let now = Utc::now().timestamp();
    db.lock().unwrap().execute(
        "UPDATE work_session SET ended_at = ?1, duration_secs = ?2 WHERE id = ?3",
        params![now, total_secs, session_id],
    )?;
    Ok(())
}

pub fn record_event(
    db: &Db,
    session_id: Option<i64>,
    kind: ReminderKind,
    action: ReminderAction,
) -> Result<()> {
    let now = Utc::now().timestamp();
    db.lock().unwrap().execute(
        "INSERT INTO reminder_event (session_id, kind, triggered_at, action) VALUES (?1, ?2, ?3, ?4)",
        params![session_id, kind.db_key(), now, action.as_str()],
    )?;
    Ok(())
}

/// 提交一笔当日汇总累加
pub fn upsert_today(db: &Db, summary: &DailySummary) -> Result<()> {
    db.lock().unwrap().execute(
        "INSERT INTO daily_summary
            (date, work_seconds, water_count, stand_count, eye_break_count, neck_count, pomodoros, big_breaks)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(date) DO UPDATE SET
             work_seconds = excluded.work_seconds,
             water_count = excluded.water_count,
             stand_count = excluded.stand_count,
             eye_break_count = excluded.eye_break_count,
             neck_count = excluded.neck_count,
             pomodoros = excluded.pomodoros,
             big_breaks = excluded.big_breaks",
        params![
            summary.date,
            summary.work_seconds,
            summary.water_count,
            summary.stand_count,
            summary.eye_break_count,
            summary.neck_count,
            summary.pomodoros,
            summary.big_breaks,
        ],
    )?;
    Ok(())
}

/// 读取今日汇总
pub fn get_today(db: &Db) -> Result<DailySummary> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let conn = db.lock().unwrap();
    let row = conn
        .query_row(
            "SELECT date, work_seconds, water_count, stand_count, eye_break_count, neck_count, pomodoros, big_breaks
             FROM daily_summary WHERE date = ?1",
            params![today],
            |r| {
                Ok(DailySummary {
                    date: r.get(0)?,
                    work_seconds: r.get(1)?,
                    water_count: r.get(2)?,
                    stand_count: r.get(3)?,
                    eye_break_count: r.get(4)?,
                    neck_count: r.get(5)?,
                    pomodoros: r.get(6)?,
                    big_breaks: r.get(7)?,
                })
            },
        )
        .optional()?;
    Ok(row.unwrap_or(DailySummary {
        date: today,
        ..Default::default()
    }))
}

/// 取最近 N 天的趋势（按日期升序）
pub fn recent_days(db: &Db, days: i64) -> Result<Vec<DailyPoint>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT date, work_seconds,
            (water_count + stand_count + eye_break_count + neck_count + pomodoros + big_breaks) AS events
         FROM daily_summary
         ORDER BY date DESC
         LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![days], |r| {
            let date_str: String = r.get(0)?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .unwrap_or_else(|_| Local::now().date_naive());
            Ok(DailyPoint {
                date,
                work_seconds: r.get(1)?,
                completed_events: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows.into_iter().rev().collect())
}

/// 取一段时间内每个提醒种类的完成次数
pub fn kind_distribution(db: &Db, days: i64) -> Result<Vec<(String, i64)>> {
    let since = (Utc::now() - chrono::Duration::days(days)).timestamp();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT kind, COUNT(*) FROM reminder_event
         WHERE triggered_at >= ?1 AND action = 'completed'
         GROUP BY kind ORDER BY 2 DESC",
    )?;
    let rows = stmt
        .query_map(params![since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// 当前是否有未结束的会话
pub fn last_open_session(db: &Db) -> Result<Option<i64>> {
    let conn = db.lock().unwrap();
    let id = conn
        .query_row(
            "SELECT id FROM work_session WHERE ended_at IS NULL ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    Ok(id)
}

/// 把 unix 秒转本地时间字符串（调试用）
#[allow(dead_code)]
pub fn ts_to_local(ts: i64) -> String {
    let utc = Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now);
    let local: DateTime<Local> = utc.with_timezone(&Local);
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_session_and_event() -> Result<()> {
        let db = super::super::open_in_memory()?;
        let sid = start_session(&db)?;
        record_event(&db, Some(sid), ReminderKind::Water, ReminderAction::Completed)?;
        record_event(&db, Some(sid), ReminderKind::Eyes, ReminderAction::Completed)?;
        end_session(&db, sid, 3600)?;

        let summary = DailySummary {
            date: Local::now().format("%Y-%m-%d").to_string(),
            work_seconds: 3600,
            water_count: 1,
            eye_break_count: 1,
            ..Default::default()
        };
        upsert_today(&db, &summary)?;
        let got = get_today(&db)?;
        assert_eq!(got.water_count, 1);
        assert_eq!(got.eye_break_count, 1);
        assert_eq!(got.work_seconds, 3600);
        Ok(())
    }

    #[test]
    fn no_open_session_initially() -> Result<()> {
        let db = super::super::open_in_memory()?;
        assert!(last_open_session(&db)?.is_none());
        let sid = start_session(&db)?;
        assert_eq!(last_open_session(&db)?, Some(sid));
        end_session(&db, sid, 100)?;
        assert!(last_open_session(&db)?.is_none());
        Ok(())
    }
}
