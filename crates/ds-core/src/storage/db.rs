use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::migrations;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single recorded command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    pub id: i64,
    pub command: String,
    pub directory: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub project_context: Option<String>,
}

/// Aggregate statistics for commands within a project directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStats {
    pub total_commands: u64,
    pub unique_commands: u64,
    pub error_rate: f64,
    pub most_used: Vec<(String, u64)>,
}

/// A cached context snapshot for a directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedContext {
    pub directory: String,
    pub context_json: String,
    pub cached_at: DateTime<Utc>,
}

/// Aggregate AI usage statistics over a period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiUsageStats {
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_cost: f64,
    pub by_provider: HashMap<String, ProviderUsage>,
}

/// Per-provider usage breakdown.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost: f64,
}

/// Time period for querying AI usage.
#[derive(Debug, Clone, Copy)]
pub enum UsagePeriod {
    Today,
    Week,
    Month,
    All,
}

// ---------------------------------------------------------------------------
// Database wrapper
// ---------------------------------------------------------------------------

/// Thin wrapper around a `rusqlite::Connection` providing typed helpers
/// for DeftShell's storage needs.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) a SQLite database at `path` and run all pending
    /// migrations so the schema is always up to date.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create database directory: {}", parent.display())
            })?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;

        // Restrict file permissions on Unix (owner read/write only).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if path.exists() {
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
            }
        }

        // Basic pragmas for performance and safety.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .context("Failed to set database pragmas")?;

        migrations::run_migrations(&conn).context("Failed to run database migrations")?;

        Ok(Self { conn })
    }

    /// Open an in-memory database (useful for tests).
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to open in-memory database")?;
        migrations::run_migrations(&conn)?;
        Ok(Self { conn })
    }

    // -----------------------------------------------------------------------
    // Command history
    // -----------------------------------------------------------------------

    /// Record a command execution.
    pub fn record_command(
        &self,
        cmd: &str,
        dir: &str,
        exit_code: Option<i32>,
        duration_ms: Option<i64>,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO command_history (command, directory, exit_code, duration_ms)
                 VALUES (?1, ?2, ?3, ?4)",
                params![cmd, dir, exit_code, duration_ms],
            )
            .context("Failed to record command")?;
        Ok(())
    }

    /// Retrieve the most recent commands executed in `project_dir`, up to
    /// `limit` rows.
    pub fn get_recent_commands(&self, project_dir: &str, limit: u32) -> Result<Vec<CommandRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, command, directory, exit_code, duration_ms, timestamp, project_context
             FROM command_history
             WHERE directory = ?1
             ORDER BY timestamp DESC, id DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![project_dir, limit], |row| {
            let ts_str: String = row.get(5)?;
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .or_else(|_| {
                    // SQLite default format: "YYYY-MM-DD HH:MM:SS"
                    chrono::NaiveDateTime::parse_from_str(&ts_str, "%Y-%m-%d %H:%M:%S")
                        .map(|naive| naive.and_utc().fixed_offset())
                })
                .unwrap_or_else(|_| Utc::now().fixed_offset());

            Ok(CommandRecord {
                id: row.get(0)?,
                command: row.get(1)?,
                directory: row.get(2)?,
                exit_code: row.get(3)?,
                duration_ms: row.get(4)?,
                timestamp: timestamp.with_timezone(&Utc),
                project_context: row.get(6)?,
            })
        })?;

        let mut commands = Vec::new();
        for row in rows {
            commands.push(row.context("Failed to read command record")?);
        }
        Ok(commands)
    }

    /// Compute aggregate statistics for commands.
    ///
    /// Pass `"*"` for `project_dir` to aggregate across all directories.
    pub fn get_command_stats(&self, project_dir: &str) -> Result<CommandStats> {
        let all = project_dir == "*";

        let total_commands: u64 = if all {
            self.conn
                .query_row("SELECT COUNT(*) FROM command_history", [], |row| row.get(0))?
        } else {
            self.conn.query_row(
                "SELECT COUNT(*) FROM command_history WHERE directory = ?1",
                [project_dir],
                |row| row.get(0),
            )?
        };

        let unique_commands: u64 = if all {
            self.conn.query_row(
                "SELECT COUNT(DISTINCT command) FROM command_history",
                [],
                |row| row.get(0),
            )?
        } else {
            self.conn.query_row(
                "SELECT COUNT(DISTINCT command) FROM command_history WHERE directory = ?1",
                [project_dir],
                |row| row.get(0),
            )?
        };

        let error_count: u64 = if all {
            self.conn.query_row(
                "SELECT COUNT(*) FROM command_history
                 WHERE exit_code IS NOT NULL AND exit_code != 0",
                [],
                |row| row.get(0),
            )?
        } else {
            self.conn.query_row(
                "SELECT COUNT(*) FROM command_history
                 WHERE directory = ?1 AND exit_code IS NOT NULL AND exit_code != 0",
                [project_dir],
                |row| row.get(0),
            )?
        };

        let error_rate = if total_commands > 0 {
            error_count as f64 / total_commands as f64
        } else {
            0.0
        };

        let most_used: Vec<(String, u64)> = if all {
            let mut stmt = self.conn.prepare(
                "SELECT command, COUNT(*) as cnt
                 FROM command_history
                 GROUP BY command
                 ORDER BY cnt DESC
                 LIMIT 10",
            )?;
            let results: Vec<(String, u64)> = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();
            results
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT command, COUNT(*) as cnt
                 FROM command_history
                 WHERE directory = ?1
                 GROUP BY command
                 ORDER BY cnt DESC
                 LIMIT 10",
            )?;
            let results: Vec<(String, u64)> = stmt
                .query_map([project_dir], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();
            results
        };

        Ok(CommandStats {
            total_commands,
            unique_commands,
            error_rate,
            most_used,
        })
    }

    // -----------------------------------------------------------------------
    // Context cache
    // -----------------------------------------------------------------------

    /// Cache (upsert) a context JSON blob for a directory.
    pub fn cache_context(&self, dir: &str, context_json: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO context_cache (directory, context_json, cached_at)
                 VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(directory) DO UPDATE SET
                     context_json = excluded.context_json,
                     cached_at = excluded.cached_at",
                params![dir, context_json],
            )
            .context("Failed to cache context")?;
        Ok(())
    }

    /// Retrieve the cached context for a directory, if any.
    pub fn get_cached_context(&self, dir: &str) -> Result<Option<CachedContext>> {
        let mut stmt = self.conn.prepare(
            "SELECT directory, context_json, cached_at
             FROM context_cache
             WHERE directory = ?1",
        )?;

        let result = stmt.query_row([dir], |row| {
            let ts_str: String = row.get(2)?;
            let cached_at = DateTime::parse_from_rfc3339(&ts_str)
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&ts_str, "%Y-%m-%d %H:%M:%S")
                        .map(|naive| naive.and_utc().fixed_offset())
                })
                .unwrap_or_else(|_| Utc::now().fixed_offset());

            Ok(CachedContext {
                directory: row.get(0)?,
                context_json: row.get(1)?,
                cached_at: cached_at.with_timezone(&Utc),
            })
        });

        match result {
            Ok(ctx) => Ok(Some(ctx)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("Failed to read cached context"),
        }
    }

    // -----------------------------------------------------------------------
    // AI usage tracking
    // -----------------------------------------------------------------------

    /// Record a single AI request's token usage and cost.
    pub fn record_ai_usage(
        &self,
        provider: &str,
        tokens_in: u64,
        tokens_out: u64,
        cost: f64,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO ai_usage (provider, tokens_in, tokens_out, cost_cents)
                 VALUES (?1, ?2, ?3, ?4)",
                params![provider, tokens_in, tokens_out, cost],
            )
            .context("Failed to record AI usage")?;
        Ok(())
    }

    /// Get aggregate AI usage statistics for a given time period.
    pub fn get_ai_usage(&self, period: UsagePeriod) -> Result<AiUsageStats> {
        let time_filter = match period {
            UsagePeriod::Today => "AND timestamp >= date('now')",
            UsagePeriod::Week => "AND timestamp >= date('now', '-7 days')",
            UsagePeriod::Month => "AND timestamp >= date('now', '-30 days')",
            UsagePeriod::All => "",
        };

        let query = format!(
            "SELECT provider, SUM(tokens_in), SUM(tokens_out), SUM(cost_cents)
             FROM ai_usage
             WHERE 1=1 {time_filter}
             GROUP BY provider"
        );

        let mut stmt = self.conn.prepare(&query)?;

        let mut total_tokens_in: u64 = 0;
        let mut total_tokens_out: u64 = 0;
        let mut total_cost: f64 = 0.0;
        let mut by_provider = HashMap::new();

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u64>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

        for row in rows {
            let (provider, t_in, t_out, cost) = row.context("Failed to read AI usage row")?;
            total_tokens_in += t_in;
            total_tokens_out += t_out;
            total_cost += cost;
            by_provider.insert(
                provider,
                ProviderUsage {
                    tokens_in: t_in,
                    tokens_out: t_out,
                    cost,
                },
            );
        }

        Ok(AiUsageStats {
            total_tokens_in,
            total_tokens_out,
            total_cost,
            by_provider,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().expect("Failed to create test database")
    }

    #[test]
    fn test_record_and_retrieve_commands() {
        let db = test_db();
        db.record_command("cargo build", "/home/user/project", Some(0), Some(1200))
            .unwrap();
        db.record_command("cargo test", "/home/user/project", Some(1), Some(3400))
            .unwrap();
        db.record_command("ls", "/home/user/other", Some(0), Some(5))
            .unwrap();

        let commands = db.get_recent_commands("/home/user/project", 10).unwrap();
        assert_eq!(commands.len(), 2);
        // Most recent first
        assert_eq!(commands[0].command, "cargo test");
        assert_eq!(commands[1].command, "cargo build");
    }

    #[test]
    fn test_command_stats() {
        let db = test_db();
        let dir = "/home/user/project";
        db.record_command("cargo build", dir, Some(0), Some(100))
            .unwrap();
        db.record_command("cargo build", dir, Some(0), Some(120))
            .unwrap();
        db.record_command("cargo test", dir, Some(1), Some(200))
            .unwrap();
        db.record_command("cargo test", dir, Some(0), Some(180))
            .unwrap();

        let stats = db.get_command_stats(dir).unwrap();
        assert_eq!(stats.total_commands, 4);
        assert_eq!(stats.unique_commands, 2);
        assert!((stats.error_rate - 0.25).abs() < f64::EPSILON);
        assert_eq!(stats.most_used.len(), 2);
    }

    #[test]
    fn test_context_cache_upsert() {
        let db = test_db();
        let dir = "/home/user/project";

        assert!(db.get_cached_context(dir).unwrap().is_none());

        db.cache_context(dir, r#"{"lang":"rust"}"#).unwrap();
        let ctx = db.get_cached_context(dir).unwrap().expect("should exist");
        assert_eq!(ctx.context_json, r#"{"lang":"rust"}"#);

        // Upsert overwrites
        db.cache_context(dir, r#"{"lang":"python"}"#).unwrap();
        let ctx = db.get_cached_context(dir).unwrap().expect("should exist");
        assert_eq!(ctx.context_json, r#"{"lang":"python"}"#);
    }

    #[test]
    fn test_ai_usage_tracking() {
        let db = test_db();
        db.record_ai_usage("openai", 500, 200, 0.015).unwrap();
        db.record_ai_usage("openai", 300, 100, 0.010).unwrap();
        db.record_ai_usage("ollama", 1000, 500, 0.0).unwrap();

        let stats = db.get_ai_usage(UsagePeriod::All).unwrap();
        assert_eq!(stats.total_tokens_in, 1800);
        assert_eq!(stats.total_tokens_out, 800);
        assert!((stats.total_cost - 0.025).abs() < 1e-9);
        assert_eq!(stats.by_provider.len(), 2);

        let openai = stats.by_provider.get("openai").unwrap();
        assert_eq!(openai.tokens_in, 800);
        assert_eq!(openai.tokens_out, 300);
    }

    #[test]
    fn test_recent_commands_limit() {
        let db = test_db();
        let dir = "/project";
        for i in 0..20 {
            db.record_command(&format!("cmd-{i}"), dir, Some(0), Some(10))
                .unwrap();
        }
        let commands = db.get_recent_commands(dir, 5).unwrap();
        assert_eq!(commands.len(), 5);
    }
}
