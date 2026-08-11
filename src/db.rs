use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;

pub type Db = Arc<Mutex<Connection>>;

pub fn open_db(path: &Path) -> Result<Db> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create data dir {}", parent.display()))?;
    }

    let conn = Connection::open(path).with_context(|| format!("open {}", path.display()))?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA temp_store = MEMORY;
        "#,
    )?;

    migrate(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL
        );
        "#,
    )?;

    // Legacy DBs created before schema_migrations: treat 001 as already applied
    // when core tables exist, so we don't re-run init side effects unnecessarily.
    let has_agents: i64 = conn.query_row(
        "SELECT COUNT(1) FROM sqlite_master WHERE type='table' AND name='agents'",
        [],
        |r| r.get(0),
    )?;
    if has_agents > 0 {
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(1, ?1)",
            params![chrono::Utc::now().to_rfc3339()],
        )?;
    }

    apply_migration(conn, 1, include_str!("../migrations/001_init.sql"))?;
    apply_migration(conn, 2, include_str!("../migrations/002_agents_by_host.sql"))?;
    apply_migration(conn, 3, include_str!("../migrations/003_agent_connect.sql"))?;
    Ok(())
}

fn apply_migration(conn: &Connection, version: i64, sql: &str) -> Result<()> {
    let applied: i64 = conn.query_row(
        "SELECT COUNT(1) FROM schema_migrations WHERE version = ?1",
        params![version],
        |r| r.get(0),
    )?;
    if applied > 0 {
        return Ok(());
    }
    conn.execute_batch(sql)
        .with_context(|| format!("run migration {version}"))?;
    conn.execute(
        "INSERT INTO schema_migrations(version, applied_at) VALUES(?1, ?2)",
        params![version, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn setting_get(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn setting_set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
