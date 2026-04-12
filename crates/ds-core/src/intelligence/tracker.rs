use crate::storage::db::Database;
use anyhow::Result;

pub struct CommandTracker<'a> {
    db: &'a Database,
}

impl<'a> CommandTracker<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Record a command execution
    pub fn track(
        &self,
        command: &str,
        directory: &str,
        exit_code: i32,
        duration_ms: u64,
    ) -> Result<()> {
        let sanitized = Self::sanitize_command(command);
        self.db.record_command(
            &sanitized,
            directory,
            Some(exit_code),
            Some(duration_ms as i64),
        )?;
        Ok(())
    }

    /// Get recent commands for a project directory
    pub fn recent_commands(
        &self,
        directory: &str,
        limit: u32,
    ) -> Result<Vec<crate::storage::db::CommandRecord>> {
        self.db.get_recent_commands(directory, limit)
    }

    /// Sanitize command to remove sensitive arguments
    fn sanitize_command(command: &str) -> String {
        let mut sanitized = command.to_string();

        // Patterns to redact
        let sensitive_patterns = [
            // Environment variable assignments with sensitive names
            (
                regex::Regex::new(r"(?i)(password|secret|token|api_key|apikey|auth)\s*=\s*\S+")
                    .unwrap(),
                "$1=***",
            ),
            // -p/--password flags
            (
                regex::Regex::new(r"(?i)(-p\s+|--password[= ]\s*)\S+").unwrap(),
                "$1***",
            ),
            // Bearer tokens
            (
                regex::Regex::new(r#"Bearer\s+[^\s'"]+"#).unwrap(),
                "Bearer ***",
            ),
        ];

        for (pattern, replacement) in &sensitive_patterns {
            sanitized = pattern.replace_all(&sanitized, *replacement).to_string();
        }

        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_command() {
        assert_eq!(
            CommandTracker::sanitize_command("export API_KEY=sk-12345"),
            "export API_KEY=***"
        );
        assert_eq!(
            CommandTracker::sanitize_command("curl -H 'Authorization: Bearer abc123'"),
            "curl -H 'Authorization: Bearer ***'"
        );
        assert_eq!(
            CommandTracker::sanitize_command("git push origin main"),
            "git push origin main"
        );
    }
}
