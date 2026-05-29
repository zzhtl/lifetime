// 数据库 schema 与简易 migration
// migration 通过 user_version 推进，便于将来加表/加列

use anyhow::Result;
use rusqlite::Connection;

const CURRENT_VERSION: i32 = 1;

pub fn migrate(conn: &Connection) -> Result<()> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(V1_SQL)?;
        conn.pragma_update(None, "user_version", CURRENT_VERSION)?;
    }
    Ok(())
}

const V1_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS work_session (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    duration_secs INTEGER
);

CREATE TABLE IF NOT EXISTS reminder_event (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER,
    kind TEXT NOT NULL,
    triggered_at INTEGER NOT NULL,
    action TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES work_session(id)
);

CREATE TABLE IF NOT EXISTS daily_summary (
    date TEXT PRIMARY KEY,
    work_seconds INTEGER NOT NULL DEFAULT 0,
    water_count INTEGER NOT NULL DEFAULT 0,
    stand_count INTEGER NOT NULL DEFAULT 0,
    eye_break_count INTEGER NOT NULL DEFAULT 0,
    neck_count INTEGER NOT NULL DEFAULT 0,
    pomodoros INTEGER NOT NULL DEFAULT 0,
    big_breaks INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_event_session ON reminder_event(session_id);
CREATE INDEX IF NOT EXISTS idx_event_kind_time ON reminder_event(kind, triggered_at);
"#;
