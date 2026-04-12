use anyhow::{Context, Result};
use rusqlite::Connection;

/// A single database migration.
struct Migration {
    id: u32,
    description: &'static str,
    sql: &'static str,
}

/// All migrations in order. New migrations should be appended to the end
/// with the next sequential id.
const MIGRATIONS: &[Migration] = &[
    Migration {
        id: 1,
        description: "Create command_history table",
        sql: "
            CREATE TABLE IF NOT EXISTS command_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command TEXT NOT NULL,
                directory TEXT NOT NULL,
                exit_code INTEGER,
                duration_ms INTEGER,
                project_context TEXT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_command_history_directory
                ON command_history(directory);
            CREATE INDEX IF NOT EXISTS idx_command_history_timestamp
                ON command_history(timestamp);
        ",
    },
    Migration {
        id: 2,
        description: "Create context_cache table",
        sql: "
            CREATE TABLE IF NOT EXISTS context_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                directory TEXT NOT NULL UNIQUE,
                context_json TEXT NOT NULL,
                cached_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_context_cache_directory
                ON context_cache(directory);
        ",
    },
    Migration {
        id: 3,
        description: "Create ai_usage table",
        sql: "
            CREATE TABLE IF NOT EXISTS ai_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL,
                tokens_in INTEGER NOT NULL DEFAULT 0,
                tokens_out INTEGER NOT NULL DEFAULT 0,
                cost_cents REAL NOT NULL DEFAULT 0.0,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_ai_usage_timestamp
                ON ai_usage(timestamp);
            CREATE INDEX IF NOT EXISTS idx_ai_usage_provider
                ON ai_usage(provider);
        ",
    },
    Migration {
        id: 4,
        description: "Create observations table",
        sql: "
            CREATE TABLE IF NOT EXISTS observations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                observation_type TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_observations_timestamp
                ON observations(timestamp);
        ",
    },
    Migration {
        id: 5,
        description: "Create settings table",
        sql: "
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
        ",
    },
];

/// Run all pending database migrations.
///
/// Tracks which migrations have already been applied via a `_migrations`
/// meta-table, so this is safe to call on every startup.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Ensure the migrations tracking table exists.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .context("Failed to create _migrations table")?;

    for migration in MIGRATIONS {
        let already_applied: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _migrations WHERE id = ?1",
                [migration.id],
                |row| row.get(0),
            )
            .context("Failed to check migration status")?;

        if already_applied {
            continue;
        }

        tracing::info!(
            "Applying migration {}: {}",
            migration.id,
            migration.description
        );

        // Wrap each migration in a transaction so the schema change and
        // the recording in _migrations are applied atomically.
        conn.execute_batch("BEGIN TRANSACTION;")
            .context("Failed to begin migration transaction")?;

        match (|| -> Result<()> {
            conn.execute_batch(migration.sql).with_context(|| {
                format!(
                    "Failed to apply migration {}: {}",
                    migration.id, migration.description
                )
            })?;
            conn.execute(
                "INSERT INTO _migrations (id, description) VALUES (?1, ?2)",
                rusqlite::params![migration.id, migration.description],
            )
            .with_context(|| format!("Failed to record migration {}", migration.id))?;
            Ok(())
        })() {
            Ok(()) => {
                conn.execute_batch("COMMIT;")
                    .context("Failed to commit migration transaction")?;
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_run_cleanly() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Verify all tables exist by querying them.
        conn.execute_batch("SELECT 1 FROM command_history LIMIT 0")
            .unwrap();
        conn.execute_batch("SELECT 1 FROM context_cache LIMIT 0")
            .unwrap();
        conn.execute_batch("SELECT 1 FROM ai_usage LIMIT 0")
            .unwrap();
        conn.execute_batch("SELECT 1 FROM observations LIMIT 0")
            .unwrap();
        conn.execute_batch("SELECT 1 FROM settings LIMIT 0")
            .unwrap();
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as u32);
    }
}
