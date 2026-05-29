// SQLite 数据层
// 三张表：work_session / reminder_event / daily_summary
// 用 bundled SQLite，跨平台零外部依赖

mod queries;
mod schema;

#[allow(unused_imports)]
pub use queries::*;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// 数据库句柄
pub type Db = Arc<Mutex<Connection>>;

/// 打开数据库并跑 migration
pub fn open(path: &Path) -> Result<Db> {
    let conn = Connection::open(path).with_context(|| format!("打开数据库失败: {:?}", path))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")
        .context("设置 PRAGMA 失败")?;
    schema::migrate(&conn).context("执行 migration 失败")?;
    Ok(Arc::new(Mutex::new(conn)))
}

/// 仅用于测试的内存数据库
#[cfg(test)]
pub fn open_in_memory() -> Result<Db> {
    let conn = Connection::open_in_memory()?;
    schema::migrate(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}
