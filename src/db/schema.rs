// 数据库 schema 与简易 migration
// migration 通过 user_version 推进，便于将来加表/加列

use anyhow::Result;
use rusqlite::Connection;

const CURRENT_VERSION: i32 = 2;

pub fn migrate(conn: &Connection) -> Result<()> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    // 增量迁移：每个版本一个区块，按当前版本逐级补齐，最后统一推进 user_version。
    // 旧库（v1）只会补跑 V2；全新库会依次跑 V1、V2。
    if version < 1 {
        conn.execute_batch(V1_SQL)?;
    }
    if version < 2 {
        conn.execute_batch(V2_SQL)?;
    }
    if version < CURRENT_VERSION {
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

// v2：养生修炼「打卡」日志。累计打卡数 = 修为值，驱动修为境界（见 practices::realm_progress）。
// UNIQUE(logged_date, title) + INSERT OR IGNORE 保证同一天同一功法只记一次。
const V2_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS practice_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    logged_date TEXT NOT NULL,
    category    TEXT NOT NULL,
    title       TEXT NOT NULL,
    logged_at   INTEGER NOT NULL,
    UNIQUE(logged_date, title)
);

CREATE INDEX IF NOT EXISTS idx_practice_log_date ON practice_log(logged_date);
"#;
