use anyhow::Result;
use std::path::Path;
use tracing::debug;

use crate::config::schema::AiContextConfig;
use crate::context::stack_profile::StackProfile;

/// Builds a contextual prompt string from project state to feed into an AI
/// request as part of the system prompt or user context.
pub struct AiContextBuilder;

impl AiContextBuilder {
    /// Assemble a context string for the AI from project state.
    ///
    /// The budget is approximately 30 % of `context_window_chars` (estimated as
    /// `max_tokens * 4` characters).  Sections are included in priority order
    /// and truncated to fit within the budget.
    ///
    /// Priority (highest first):
    /// 1. Last error output
    /// 2. Key config / project file contents
    /// 3. Git status
    /// 4. Recent git commits
    /// 5. Directory listing
    /// 6. Stack profile summary
    pub fn build(
        stack_profile: &StackProfile,
        dir: &Path,
        config: &AiContextConfig,
        context_window_tokens: u32,
        last_error: Option<&str>,
    ) -> Result<String> {
        // 30% of context window, estimated at ~4 chars per token.
        let budget = (context_window_tokens as usize) * 4 * 30 / 100;
        let mut sections: Vec<String> = Vec::new();
        let mut used: usize = 0;

        // Helper: push a section if budget allows, truncating as needed.
        let mut push = |header: &str, body: String| {
            if used >= budget || body.is_empty() {
                return;
            }
            let available = budget - used;
            let section = format!("## {header}\n{body}\n");
            if section.len() <= available {
                used += section.len();
                sections.push(section);
            } else {
                // Truncate the body to fit.
                let truncated: String =
                    section.chars().take(available.saturating_sub(20)).collect();
                let section = format!("{truncated}\n... (truncated)\n");
                used += section.len();
                sections.push(section);
            }
        };

        // --- 1. Last error output -------------------------------------------
        if let Some(err) = last_error {
            push("Last Error Output", err.to_string());
        }

        // --- 2. Key config file contents ------------------------------------
        let config_files = Self::discover_config_files(dir, config);
        for path in config_files {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                let relative = path
                    .strip_prefix(dir)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                push(&format!("File: {relative}"), contents);
            }
        }

        // --- 3. Git status ---------------------------------------------------
        let git_status = Self::git_status(dir);
        push("Git Status", git_status);

        // --- 4. Recent git commits -------------------------------------------
        let git_log = Self::git_log(dir, 10);
        push("Recent Commits", git_log);

        // --- 5. Directory listing --------------------------------------------
        let dir_listing = Self::directory_listing(dir, config);
        push("Directory Structure", dir_listing);

        // --- 6. Stack profile summary ----------------------------------------
        let profile_summary = Self::profile_summary(stack_profile);
        push("Stack Profile", profile_summary);

        debug!(
            used_chars = used,
            budget_chars = budget,
            sections = sections.len(),
            "context built"
        );

        Ok(sections.join("\n"))
    }

    // -- section builders ---------------------------------------------------

    fn profile_summary(profile: &StackProfile) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push(format!("Project: {}", profile.project.name));

        if let Some(ref lang) = profile.stack.primary_language {
            lines.push(format!("Language: {lang}"));
        }
        if let Some(ref runtime) = profile.stack.runtime {
            let version = profile
                .stack
                .runtime_version
                .as_deref()
                .unwrap_or("unknown");
            lines.push(format!("Runtime: {runtime} {version}"));
        }
        if let Some(ref framework) = profile.stack.framework {
            lines.push(format!("Framework: {framework}"));
        }
        if let Some(ref pm) = profile.stack.package_manager {
            lines.push(format!("Package manager: {pm}"));
        }
        if let Some(ref tr) = profile.stack.test_runner {
            lines.push(format!("Test runner: {tr}"));
        }
        if profile.infrastructure.containerized {
            lines.push("Containerized: yes".to_string());
        }
        if let Some(ref db) = profile.services.database {
            lines.push(format!("Database: {db}"));
        }

        lines.join("\n")
    }

    fn directory_listing(dir: &Path, config: &AiContextConfig) -> String {
        let mut entries: Vec<String> = Vec::new();

        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return String::new(),
        };

        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if Self::is_excluded(&name, config) {
                continue;
            }
            let kind = if entry.path().is_dir() { "dir" } else { "file" };
            entries.push(format!("  {name} ({kind})"));
        }

        entries.sort();
        entries.join("\n")
    }

    fn git_status(dir: &Path) -> String {
        let output = std::process::Command::new("git")
            .args(["status", "--short"])
            .current_dir(dir)
            .output();

        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => String::new(),
        }
    }

    fn git_log(dir: &Path, n: usize) -> String {
        let output = std::process::Command::new("git")
            .args(["log", &format!("-{n}"), "--oneline", "--no-decorate"])
            .current_dir(dir)
            .output();

        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => String::new(),
        }
    }

    /// Discover well-known config files that are not excluded.
    fn discover_config_files(dir: &Path, config: &AiContextConfig) -> Vec<std::path::PathBuf> {
        let candidates = [
            "package.json",
            "Cargo.toml",
            "pyproject.toml",
            "go.mod",
            "Makefile",
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.yaml",
            ".deftshell.toml",
            "tsconfig.json",
            "requirements.txt",
            "Gemfile",
            "build.gradle",
            "pom.xml",
        ];

        let mut found = Vec::new();

        // Include explicitly listed files first.
        for pattern in &config.include_files {
            let path = dir.join(pattern);
            if path.exists() && !Self::is_excluded(pattern, config) {
                found.push(path);
            }
        }

        // Then well-known files.
        for name in &candidates {
            let path = dir.join(name);
            if path.exists() && !Self::is_excluded(name, config) && !found.contains(&path) {
                found.push(path);
            }
        }

        found
    }

    /// Check if a filename should be excluded based on the context config.
    fn is_excluded(name: &str, config: &AiContextConfig) -> bool {
        for pattern in &config.exclude_files {
            if Self::glob_match(pattern, name) {
                return true;
            }
        }
        for pattern in &config.exclude_patterns {
            if name.to_lowercase().contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// Minimal glob matching supporting leading `*` and trailing `*`.
    fn glob_match(pattern: &str, name: &str) -> bool {
        if pattern == name {
            return true;
        }
        if let Some(suffix) = pattern.strip_prefix('*') {
            return name.ends_with(suffix);
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return name.starts_with(prefix);
        }
        if let Some(prefix) = pattern.strip_suffix('/') {
            return name == prefix || name.starts_with(&format!("{prefix}/"));
        }
        false
    }
}
