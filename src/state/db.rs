use std::path::Path;

use anyhow::Context;
use tokio_rusqlite::Connection;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_core", include_str!("../../migrations/0001_core.sql")),
    (
        "0002_watch",
        include_str!("../../migrations/0002_watch.sql"),
    ),
    (
        "0003_indexes",
        include_str!("../../migrations/0003_indexes.sql"),
    ),
    (
        "0004_callback_states",
        include_str!("../../migrations/0004_callback_states.sql"),
    ),
    (
        "0005_adaptive_poll",
        include_str!("../../migrations/0005_adaptive_poll.sql"),
    ),
    (
        "0006_telegram_preferences",
        include_str!("../../migrations/0006_telegram_preferences.sql"),
    ),
    (
        "0007_watch_filters",
        include_str!("../../migrations/0007_watch_filters.sql"),
    ),
    (
        "0008_ui_requests",
        include_str!("../../migrations/0008_ui_requests.sql"),
    ),
    (
        "0009_ui_resume_kind",
        include_str!("../../migrations/0009_ui_resume_kind.sql"),
    ),
    (
        "0010_telegram_sessions",
        include_str!("../../migrations/0010_telegram_sessions.sql"),
    ),
    (
        "0011_watch_names_sync_time",
        include_str!("../../migrations/0011_watch_names_sync_time.sql"),
    ),
    (
        "0012_retry_state_and_telegram_dedup",
        include_str!("../../migrations/0012_retry_state_and_telegram_dedup.sql"),
    ),
];

#[derive(Clone)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create db dir {}", parent.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
            }
        }
        let conn = Connection::open(path)
            .await
            .with_context(|| format!("open sqlite db {}", path.display()))?;
        let db = Self { conn };
        db.configure().await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if path.exists() {
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
            }
        }
        Ok(db)
    }

    pub async fn in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory().await?;
        let db = Self { conn };
        db.configure().await?;
        Ok(db)
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        self.conn
            .call(|conn| {
                let tx = conn.transaction()?;
                tx.execute_batch(
                    "CREATE TABLE IF NOT EXISTS schema_migrations (
                        version TEXT PRIMARY KEY,
                        applied_at_ms INTEGER NOT NULL
                    );",
                )?;
                for (version, sql) in MIGRATIONS {
                    let already_applied: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                        [version],
                        |row| row.get(0),
                    )?;
                    if !already_applied {
                        tx.execute_batch(sql)?;
                        tx.execute(
                            "INSERT INTO schema_migrations (version, applied_at_ms) VALUES (?1, ?2)",
                            (*version, now_ms()),
                        )?;
                    }
                }
                tx.commit()
            })
            .await?;
        Ok(())
    }

    pub async fn ensure_owner(&self, owner_telegram_id: i64) -> anyhow::Result<()> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO authorized_users
                     (telegram_user_id, role, enabled, created_at_ms)
                     VALUES (?1, 'owner', 1, ?2)",
                    (owner_telegram_id, now_ms()),
                )?;
                Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }

    pub async fn integrity_check(&self) -> anyhow::Result<String> {
        Ok(self
            .conn
            .call(|conn| conn.query_row("PRAGMA integrity_check", [], |row| row.get(0)))
            .await?)
    }

    /// Whether a table exists — used by tests and by the GUI to detect an
    /// uninitialized database.
    pub async fn table_exists(&self, name: &str) -> anyhow::Result<bool> {
        let name = name.to_string();
        Ok(self
            .conn
            .call(move |conn| {
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [name],
                    |row| row.get(0),
                )?;
                Ok::<bool, rusqlite::Error>(exists)
            })
            .await?)
    }

    pub(crate) fn conn(&self) -> Connection {
        self.conn.clone()
    }

    async fn configure(&self) -> anyhow::Result<()> {
        self.conn
            .call(|conn| {
                conn.pragma_update(None, "foreign_keys", "ON")?;
                conn.pragma_update(None, "journal_mode", "WAL")?;
                conn.pragma_update(None, "synchronous", "NORMAL")?;
                conn.pragma_update(None, "busy_timeout", 5000)?;
                conn.pragma_update(None, "wal_autocheckpoint", 1000)?;
                Ok::<(), rusqlite::Error>(())
            })
            .await?;
        Ok(())
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(unix)]
    async fn enforces_0600_permissions_on_db_file() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = std::env::temp_dir().join(format!("502drive_test_{}", now_ms()));
        let db_path = temp_dir.join("test_state.db");
        let _db = Database::open(&db_path).await.expect("open db");
        let meta = std::fs::metadata(&db_path).expect("metadata");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
