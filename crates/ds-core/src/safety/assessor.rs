use super::interceptor::{InterceptionContext, SafetyAlert};
use super::rules::RiskLevel;

/// The result of a context-aware risk assessment.
#[derive(Debug, Clone)]
pub struct AssessedRisk {
    /// The original risk level from the matched rule.
    pub original_level: RiskLevel,
    /// The assessed risk level after applying contextual elevation.
    /// This is always >= original_level (the assessor can only elevate, never reduce).
    pub assessed_level: RiskLevel,
    /// Reasons for the assessment, including any elevation rationale.
    pub reasons: Vec<String>,
}

/// Context-aware risk assessor that can elevate risk levels based on
/// environmental factors like the current git branch, production
/// environment detection, and Kubernetes context.
pub struct RiskAssessor;

impl RiskAssessor {
    /// Create a new risk assessor.
    pub fn new() -> Self {
        Self
    }

    /// Assess the risk of a command given its initial safety alert and the
    /// current interception context.
    ///
    /// The assessor can only elevate risk levels, never reduce them. It checks
    /// several contextual factors:
    ///
    /// - Protected branch detection (main, master, production)
    /// - Production environment detection
    /// - Kubernetes production context detection
    /// - Uncommitted changes with destructive commands
    pub fn assess(
        &self,
        command: &str,
        alert: &SafetyAlert,
        context: &InterceptionContext,
    ) -> AssessedRisk {
        let original_level = alert.level;
        let mut assessed_level = original_level;
        let mut reasons = vec![alert.reason.clone()];

        // Check if on a protected branch
        if let Some(ref branch) = context.current_branch {
            if Self::is_protected_branch(branch) {
                if Self::is_git_destructive(command) {
                    let elevated = Self::elevate(assessed_level);
                    if elevated > assessed_level {
                        reasons.push(format!(
                            "Risk elevated: destructive git operation on protected branch '{branch}'"
                        ));
                        assessed_level = elevated;
                    }
                }

                if Self::is_database_command(command) {
                    let elevated = Self::elevate(assessed_level);
                    if elevated > assessed_level {
                        reasons.push(format!(
                            "Risk elevated: database operation while on protected branch '{branch}'"
                        ));
                        assessed_level = elevated;
                    }
                }
            }
        }

        // Check if in a production environment
        if context.is_production_env {
            if Self::is_database_command(command) {
                let elevated = Self::elevate(assessed_level);
                if elevated > assessed_level {
                    reasons.push(
                        "Risk elevated: database operation in production environment".to_string(),
                    );
                    assessed_level = elevated;
                }
            }

            if Self::is_infrastructure_command(command) {
                let elevated = Self::elevate(assessed_level);
                if elevated > assessed_level {
                    reasons.push(
                        "Risk elevated: infrastructure operation in production environment"
                            .to_string(),
                    );
                    assessed_level = elevated;
                }
            }
        }

        // Kubernetes production context: elevate ALL to Critical
        if let Some(ref k8s_ctx) = context.kubernetes_context {
            if Self::is_production_k8s_context(k8s_ctx) && assessed_level < RiskLevel::Critical {
                reasons.push(format!(
                    "Risk elevated to Critical: production Kubernetes context '{k8s_ctx}' detected"
                ));
                assessed_level = RiskLevel::Critical;
            }
        }

        // Uncommitted changes with destructive git operations
        if context.has_uncommitted_changes && Self::is_git_destructive(command) {
            let elevated = Self::elevate(assessed_level);
            if elevated > assessed_level {
                reasons.push(
                    "Risk elevated: destructive operation with uncommitted changes".to_string(),
                );
                assessed_level = elevated;
            }
        }

        AssessedRisk {
            original_level,
            assessed_level,
            reasons,
        }
    }

    /// Elevate a risk level by one step. Critical cannot be elevated further.
    fn elevate(level: RiskLevel) -> RiskLevel {
        match level {
            RiskLevel::Low => RiskLevel::Medium,
            RiskLevel::Medium => RiskLevel::High,
            RiskLevel::High => RiskLevel::Critical,
            RiskLevel::Critical => RiskLevel::Critical,
        }
    }

    /// Check if a branch name is a protected branch.
    fn is_protected_branch(branch: &str) -> bool {
        matches!(
            branch.to_lowercase().as_str(),
            "main" | "master" | "production" | "prod" | "release"
        )
    }

    /// Check if a command is a destructive git operation.
    fn is_git_destructive(command: &str) -> bool {
        let destructive_patterns = [
            "git push --force",
            "git push -f",
            "git reset --hard",
            "git clean -f",
            "git clean -fd",
            "git clean -df",
            "git checkout -- .",
        ];
        destructive_patterns.iter().any(|p| command.contains(p))
    }

    /// Check if a command involves database operations.
    fn is_database_command(command: &str) -> bool {
        let upper = command.to_uppercase();
        upper.contains("DROP DATABASE")
            || upper.contains("DROP TABLE")
            || upper.contains("TRUNCATE")
            || upper.contains("DELETE FROM")
            || command.contains("psql")
            || command.contains("mysql")
            || command.contains("mongo")
    }

    /// Check if a command involves infrastructure operations.
    fn is_infrastructure_command(command: &str) -> bool {
        command.contains("terraform destroy")
            || command.contains("terraform apply")
            || command.contains("kubectl delete")
            || command.contains("docker system prune")
    }

    /// Check if a Kubernetes context name suggests a production environment.
    fn is_production_k8s_context(context: &str) -> bool {
        let lower = context.to_lowercase();
        lower.contains("prod")
            || lower.contains("production")
            || lower.contains("prd")
            || lower.contains("live")
    }
}

impl Default for RiskAssessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::interceptor::SafetyAlert;
    use crate::safety::rules::RiskLevel;

    fn make_alert(level: RiskLevel, command: &str) -> SafetyAlert {
        SafetyAlert {
            level,
            command: command.to_string(),
            reason: "Test alert".to_string(),
            suggestion: None,
            context_info: None,
        }
    }

    fn default_context() -> InterceptionContext {
        InterceptionContext::default()
    }

    #[test]
    fn test_no_elevation_in_safe_context() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::High, "git push --force origin feature");
        let ctx = default_context();

        let result = assessor.assess("git push --force origin feature", &alert, &ctx);
        assert_eq!(result.original_level, RiskLevel::High);
        assert_eq!(result.assessed_level, RiskLevel::High);
    }

    #[test]
    fn test_elevate_git_on_protected_branch() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::High, "git push --force origin main");
        let ctx = InterceptionContext {
            current_branch: Some("main".to_string()),
            ..default_context()
        };

        let result = assessor.assess("git push --force origin main", &alert, &ctx);
        assert_eq!(result.original_level, RiskLevel::High);
        assert_eq!(result.assessed_level, RiskLevel::Critical);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("protected branch")));
    }

    #[test]
    fn test_elevate_database_in_production() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::High, "psql -c 'DROP TABLE users'");
        let ctx = InterceptionContext {
            is_production_env: true,
            ..default_context()
        };

        let result = assessor.assess("psql -c 'DROP TABLE users'", &alert, &ctx);
        assert_eq!(result.original_level, RiskLevel::High);
        assert_eq!(result.assessed_level, RiskLevel::Critical);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("production environment")));
    }

    #[test]
    fn test_kubernetes_production_context_elevates_all_to_critical() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::Medium, "kubectl delete pod my-pod");
        let ctx = InterceptionContext {
            kubernetes_context: Some("prod-cluster-us-east-1".to_string()),
            ..default_context()
        };

        let result = assessor.assess("kubectl delete pod my-pod", &alert, &ctx);
        assert_eq!(result.original_level, RiskLevel::Medium);
        assert_eq!(result.assessed_level, RiskLevel::Critical);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("production Kubernetes context")));
    }

    #[test]
    fn test_uncommitted_changes_elevation() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::High, "git reset --hard HEAD~3");
        let ctx = InterceptionContext {
            has_uncommitted_changes: true,
            ..default_context()
        };

        let result = assessor.assess("git reset --hard HEAD~3", &alert, &ctx);
        assert_eq!(result.original_level, RiskLevel::High);
        assert_eq!(result.assessed_level, RiskLevel::Critical);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("uncommitted changes")));
    }

    #[test]
    fn test_can_only_elevate_never_reduce() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::Critical, "rm -rf /");
        let ctx = default_context(); // No escalation factors

        let result = assessor.assess("rm -rf /", &alert, &ctx);
        assert_eq!(result.original_level, RiskLevel::Critical);
        assert_eq!(result.assessed_level, RiskLevel::Critical);
    }

    #[test]
    fn test_critical_cannot_elevate_further() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::Critical, "git push --force origin main");
        let ctx = InterceptionContext {
            current_branch: Some("main".to_string()),
            is_production_env: true,
            has_uncommitted_changes: true,
            kubernetes_context: Some("production".to_string()),
        };

        let result = assessor.assess("git push --force origin main", &alert, &ctx);
        assert_eq!(result.assessed_level, RiskLevel::Critical);
    }

    #[test]
    fn test_multiple_elevation_reasons() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::Medium, "git checkout -- .");
        let ctx = InterceptionContext {
            current_branch: Some("main".to_string()),
            has_uncommitted_changes: true,
            ..default_context()
        };

        let result = assessor.assess("git checkout -- .", &alert, &ctx);
        // Should have been elevated at least once
        assert!(result.assessed_level > result.original_level);
        assert!(result.reasons.len() > 1);
    }

    #[test]
    fn test_non_production_k8s_context_no_elevation() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::Medium, "kubectl delete pod test-pod");
        let ctx = InterceptionContext {
            kubernetes_context: Some("dev-cluster".to_string()),
            ..default_context()
        };

        let result = assessor.assess("kubectl delete pod test-pod", &alert, &ctx);
        assert_eq!(result.assessed_level, result.original_level);
    }

    #[test]
    fn test_terraform_in_production() {
        let assessor = RiskAssessor::new();
        let alert = make_alert(RiskLevel::High, "terraform destroy");
        let ctx = InterceptionContext {
            is_production_env: true,
            ..default_context()
        };

        let result = assessor.assess("terraform destroy", &alert, &ctx);
        assert_eq!(result.assessed_level, RiskLevel::Critical);
        assert!(result.reasons.iter().any(|r| r.contains("infrastructure")));
    }

    #[test]
    fn test_protected_branches() {
        let assessor = RiskAssessor::new();
        let branches = ["main", "master", "production", "prod", "release"];
        let alert = make_alert(RiskLevel::High, "git push --force");

        for branch in &branches {
            let ctx = InterceptionContext {
                current_branch: Some(branch.to_string()),
                ..default_context()
            };
            let result = assessor.assess("git push --force", &alert, &ctx);
            assert_eq!(
                result.assessed_level,
                RiskLevel::Critical,
                "Expected elevation on branch '{branch}'"
            );
        }
    }
}
