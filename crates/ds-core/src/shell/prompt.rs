use crate::config::{PromptConfig, PromptTheme, ShellType};
use crate::context::stack_profile::StackProfile;
use anyhow::Result;
use std::path::Path;

/// Data collected for prompt rendering
#[derive(Debug, Default)]
pub struct PromptData {
    pub cwd: String,
    pub cwd_short: String,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
    pub git_ahead: u32,
    pub git_behind: u32,
    pub git_stash_count: u32,
    pub stack_name: Option<String>,
    pub stack_icon: Option<String>,
    pub environment: Option<String>,
    pub running_services: u32,
    pub ai_connected: bool,
    pub last_exit_code: i32,
    pub last_duration_ms: u64,
    pub aws_profile: Option<String>,
    pub k8s_context: Option<String>,
    pub k8s_namespace: Option<String>,
    pub runtime_version: Option<String>,
}

pub struct PromptRenderer {
    config: PromptConfig,
}

impl PromptRenderer {
    pub fn new(config: PromptConfig) -> Self {
        Self { config }
    }

    /// Collect prompt data from the current environment
    pub fn collect_data(
        &self,
        dir: &Path,
        exit_code: i32,
        duration_ms: u64,
        profile: Option<&StackProfile>,
    ) -> PromptData {
        let mut data = PromptData {
            cwd: dir.to_string_lossy().to_string(),
            cwd_short: Self::shorten_path(dir),
            last_exit_code: exit_code,
            last_duration_ms: duration_ms,
            ..Default::default()
        };

        // Git info
        if self.config.show_git {
            if let Ok(info) = Self::get_git_info(dir) {
                data.git_branch = info.branch;
                data.git_dirty = info.dirty;
                data.git_ahead = info.ahead;
                data.git_behind = info.behind;
                data.git_stash_count = info.stash_count;
            }
        }

        // Stack info
        if self.config.show_stack {
            if let Some(profile) = profile {
                data.stack_name = profile
                    .stack
                    .framework
                    .clone()
                    .or(profile.stack.primary_language.clone());
                data.stack_icon = Self::get_stack_icon(
                    profile.stack.framework.as_deref(),
                    profile.stack.primary_language.as_deref(),
                );
                data.runtime_version = profile.stack.runtime_version.clone();
            }
        }

        // Environment
        if self.config.show_env {
            data.environment = Self::detect_environment();
        }

        // AWS profile
        if self.config.show_aws_profile {
            data.aws_profile = std::env::var("AWS_PROFILE").ok();
        }

        // Kubernetes context
        if self.config.show_kubernetes {
            data.k8s_context = Self::get_k8s_context();
        }

        data
    }

    /// Render the left prompt
    pub fn render_left(&self, data: &PromptData, shell: ShellType) -> String {
        match self.config.theme {
            PromptTheme::Default => self.render_default_left(data, shell),
            PromptTheme::Minimal => self.render_minimal_left(data, shell),
            PromptTheme::Powerline => self.render_powerline_left(data, shell),
            PromptTheme::Pure => self.render_pure_left(data, shell),
        }
    }

    /// Render the right prompt
    pub fn render_right(&self, data: &PromptData, shell: ShellType) -> String {
        if !self.config.right_prompt {
            return String::new();
        }
        match self.config.theme {
            PromptTheme::Default => self.render_default_right(data, shell),
            _ => self.render_default_right(data, shell),
        }
    }

    fn render_default_left(&self, data: &PromptData, shell: ShellType) -> String {
        let mut parts = Vec::new();

        // Exit code indicator
        if data.last_exit_code != 0 {
            parts.push(Self::colorize("x", "red", shell));
        } else {
            parts.push(Self::colorize(">", "green", shell));
        }

        // Directory
        parts.push(Self::colorize(&data.cwd_short, "cyan", shell));

        // Git branch
        if let Some(ref branch) = data.git_branch {
            let git_str = if data.git_dirty {
                format!(" {}*", branch)
            } else {
                format!(" {}", branch)
            };
            let mut extra = String::new();
            if data.git_ahead > 0 {
                extra.push_str(&format!("^{}", data.git_ahead));
            }
            if data.git_behind > 0 {
                extra.push_str(&format!("v{}", data.git_behind));
            }
            if data.git_stash_count > 0 {
                extra.push_str(&format!("${}", data.git_stash_count));
            }
            let color = if data.git_dirty { "yellow" } else { "green" };
            parts.push(Self::colorize(
                &format!("{}{}", git_str, extra),
                color,
                shell,
            ));
        }

        // Stack indicator
        if let Some(ref icon) = data.stack_icon {
            parts.push(Self::colorize(icon, "magenta", shell));
        }

        // Environment indicator
        if let Some(ref env) = data.environment {
            let color = match env.as_str() {
                "production" | "prod" => "red",
                "staging" | "stage" => "yellow",
                _ => "green",
            };
            parts.push(Self::colorize(&format!("[{}]", env), color, shell));
        }

        // Execution time (if > threshold)
        if self.config.show_execution_time
            && data.last_duration_ms > self.config.execution_time_threshold_ms
        {
            let duration = Self::format_duration(data.last_duration_ms);
            parts.push(Self::colorize(&format!("~{}", duration), "yellow", shell));
        }

        format!("{} ", parts.join(" "))
    }

    fn render_minimal_left(&self, data: &PromptData, shell: ShellType) -> String {
        let indicator = if data.last_exit_code != 0 {
            Self::colorize(">", "red", shell)
        } else {
            Self::colorize(">", "magenta", shell)
        };
        format!("{} {} ", data.cwd_short, indicator)
    }

    fn render_powerline_left(&self, data: &PromptData, shell: ShellType) -> String {
        // Simplified powerline-style
        let mut parts = Vec::new();
        parts.push(Self::colorize(
            &format!(" {} ", data.cwd_short),
            "cyan",
            shell,
        ));
        if let Some(ref branch) = data.git_branch {
            let dirty_marker = if data.git_dirty { " *" } else { "" };
            parts.push(Self::colorize(
                &format!(" {}{} ", branch, dirty_marker),
                "green",
                shell,
            ));
        }
        let prompt_char = if data.last_exit_code != 0 { "!" } else { "$" };
        parts.push(format!("{} ", prompt_char));
        parts.join("")
    }

    fn render_pure_left(&self, data: &PromptData, shell: ShellType) -> String {
        let mut lines = Vec::new();
        // First line: path and git info
        let mut first_line = Self::colorize(&data.cwd_short, "cyan", shell);
        if let Some(ref branch) = data.git_branch {
            first_line.push_str(&Self::colorize(&format!(" {}", branch), "gray", shell));
            if data.git_dirty {
                first_line.push_str(&Self::colorize("*", "yellow", shell));
            }
        }
        if data.last_duration_ms > self.config.execution_time_threshold_ms {
            first_line.push_str(&Self::colorize(
                &format!(" {}", Self::format_duration(data.last_duration_ms)),
                "yellow",
                shell,
            ));
        }
        lines.push(first_line);
        // Second line: prompt character
        let char_color = if data.last_exit_code != 0 {
            "red"
        } else {
            "magenta"
        };
        lines.push(Self::colorize(">", char_color, shell));
        format!("{}\n{} ", lines[0], lines[1])
    }

    fn render_default_right(&self, data: &PromptData, shell: ShellType) -> String {
        let mut parts = Vec::new();

        // AWS profile
        if let Some(ref profile) = data.aws_profile {
            parts.push(Self::colorize(&format!("aws:{}", profile), "yellow", shell));
        }

        // Kubernetes context
        if let Some(ref ctx) = data.k8s_context {
            let k8s_str = if let Some(ref ns) = data.k8s_namespace {
                format!("k8s:{}:{}", ctx, ns)
            } else {
                format!("k8s:{}", ctx)
            };
            parts.push(Self::colorize(&k8s_str, "cyan", shell));
        }

        // Runtime version
        if let Some(ref ver) = data.runtime_version {
            parts.push(Self::colorize(ver, "gray", shell));
        }

        // AI status
        if self.config.show_ai_status && data.ai_connected {
            parts.push(Self::colorize("AI", "magenta", shell));
        }

        parts.join(" ")
    }

    fn colorize(text: &str, color: &str, shell: ShellType) -> String {
        let (start, end) = match shell {
            ShellType::Zsh => ("%{", "%}"),
            ShellType::Bash => ("\\[", "\\]"),
            ShellType::Fish => ("", ""),
        };
        let ansi_code = match color {
            "red" => "\x1b[31m",
            "green" => "\x1b[32m",
            "yellow" => "\x1b[33m",
            "blue" => "\x1b[94m",
            "magenta" => "\x1b[35m",
            "cyan" => "\x1b[36m",
            "gray" | "grey" => "\x1b[90m",
            _ => "\x1b[0m",
        };
        let reset = "\x1b[0m";
        if matches!(shell, ShellType::Fish) {
            format!("{}{}{}", ansi_code, text, reset)
        } else {
            format!(
                "{}{}{}{}{}",
                start,
                ansi_code,
                end,
                text,
                &format!("{}{}{}", start, reset, end)
            )
        }
    }

    fn shorten_path(path: &Path) -> String {
        if let Some(home) = dirs::home_dir() {
            if let Ok(stripped) = path.strip_prefix(&home) {
                return format!("~/{}", stripped.display());
            }
        }
        path.to_string_lossy().to_string()
    }

    fn format_duration(ms: u64) -> String {
        if ms < 1000 {
            format!("{}ms", ms)
        } else if ms < 60_000 {
            format!("{:.1}s", ms as f64 / 1000.0)
        } else {
            let mins = ms / 60_000;
            let secs = (ms % 60_000) / 1000;
            format!("{}m{}s", mins, secs)
        }
    }

    fn get_stack_icon(framework: Option<&str>, language: Option<&str>) -> Option<String> {
        let icon = match framework {
            Some("next" | "nextjs") => "Next",
            Some("react") => "React",
            Some("vue") => "Vue",
            Some("angular") => "Ng",
            Some("svelte") => "Sv",
            Some("django") => "Dj",
            Some("flask") => "Fl",
            Some("fastapi") => "FA",
            Some("rails") => "Rb",
            Some("spring") => "Sp",
            _ => match language {
                Some("typescript" | "javascript") => "JS",
                Some("rust") => "Rs",
                Some("python") => "Py",
                Some("go") => "Go",
                Some("ruby") => "Rb",
                Some("java") => "Jv",
                Some("csharp") => "C#",
                Some("swift") => "Sw",
                Some("elixir") => "Ex",
                Some("dart") => "Da",
                _ => return None,
            },
        };
        Some(icon.to_string())
    }

    fn detect_environment() -> Option<String> {
        // Check common environment indicators
        if let Ok(env) = std::env::var("NODE_ENV") {
            return Some(env);
        }
        if let Ok(env) = std::env::var("RAILS_ENV") {
            return Some(env);
        }
        if let Ok(env) = std::env::var("FLASK_ENV") {
            return Some(env);
        }
        if let Ok(env) = std::env::var("APP_ENV") {
            return Some(env);
        }
        if let Ok(env) = std::env::var("ENVIRONMENT") {
            return Some(env);
        }
        None
    }

    fn get_k8s_context() -> Option<String> {
        let output = std::process::Command::new("kubectl")
            .args(["config", "current-context"])
            .output()
            .ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }
}

struct GitInfo {
    branch: Option<String>,
    dirty: bool,
    ahead: u32,
    behind: u32,
    stash_count: u32,
}

impl PromptRenderer {
    fn get_git_info(dir: &Path) -> Result<GitInfo> {
        let mut repo = git2::Repository::discover(dir)?;

        let head = repo.head().ok();
        let branch = head
            .as_ref()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let dirty = {
            let statuses = repo.statuses(Some(
                git2::StatusOptions::new()
                    .include_untracked(true)
                    .exclude_submodules(true),
            ))?;
            !statuses.is_empty()
        };

        // Ahead/behind
        let (ahead, behind) = if let Some(ref head_ref) = head {
            if let Some(oid) = head_ref.target() {
                if let Ok(branch_ref) =
                    repo.find_branch(branch.as_deref().unwrap_or(""), git2::BranchType::Local)
                {
                    if let Ok(upstream) = branch_ref.upstream() {
                        if let Some(upstream_oid) = upstream.get().target() {
                            repo.graph_ahead_behind(oid, upstream_oid).unwrap_or((0, 0))
                        } else {
                            (0, 0)
                        }
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        // Drop head to release the immutable borrow on repo before stash_foreach
        drop(head);

        // Stash count
        let mut stash_count = 0u32;
        repo.stash_foreach(|_, _, _| {
            stash_count += 1;
            true
        })
        .ok();

        Ok(GitInfo {
            branch,
            dirty,
            ahead: ahead as u32,
            behind: behind as u32,
            stash_count,
        })
    }
}
