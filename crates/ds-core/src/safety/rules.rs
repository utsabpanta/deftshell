use serde::{Deserialize, Serialize};

/// Risk level for a safety rule, ordered from lowest to highest severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

impl RiskLevel {
    /// Parse a risk level from a string (case-insensitive).
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Some(RiskLevel::Critical),
            "high" => Some(RiskLevel::High),
            "medium" => Some(RiskLevel::Medium),
            "low" => Some(RiskLevel::Low),
            _ => None,
        }
    }
}

/// A safety rule that matches commands against a regex pattern
/// and assigns a risk level.
#[derive(Debug, Clone)]
pub struct SafetyRule {
    /// Regex pattern to match against the command string.
    pub pattern: String,
    /// The risk level assigned when this rule matches.
    pub level: RiskLevel,
    /// Category of the rule (e.g., "filesystem", "git", "database").
    pub category: String,
    /// Human-readable message explaining the risk.
    pub message: String,
    /// Optional suggestion for a safer alternative.
    pub suggestion: Option<String>,
}

/// Provider of built-in safety rules.
pub struct BuiltinRules;

impl BuiltinRules {
    /// Returns all built-in safety rules across all risk levels.
    pub fn all() -> Vec<SafetyRule> {
        let mut rules = Vec::new();
        rules.extend(Self::critical_rules());
        rules.extend(Self::high_rules());
        rules.extend(Self::medium_rules());
        rules
    }

    fn critical_rules() -> Vec<SafetyRule> {
        vec![
            SafetyRule {
                pattern: r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|(-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+)/\s*$".to_string(),
                level: RiskLevel::Critical,
                category: "filesystem".to_string(),
                message: "Recursive forced deletion of the root filesystem".to_string(),
                suggestion: Some("Specify the exact directory you want to remove instead of /".to_string()),
            },
            SafetyRule {
                pattern: r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|(-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+)/\*".to_string(),
                level: RiskLevel::Critical,
                category: "filesystem".to_string(),
                message: "Recursive forced deletion of all files under root".to_string(),
                suggestion: Some("Specify the exact directory you want to remove".to_string()),
            },
            SafetyRule {
                pattern: r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|(-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+)~\s*$".to_string(),
                level: RiskLevel::Critical,
                category: "filesystem".to_string(),
                message: "Recursive forced deletion of the home directory".to_string(),
                suggestion: Some("Specify the exact subdirectory you want to remove".to_string()),
            },
            SafetyRule {
                pattern: r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|(-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+)\$HOME\b".to_string(),
                level: RiskLevel::Critical,
                category: "filesystem".to_string(),
                message: "Recursive forced deletion of $HOME directory".to_string(),
                suggestion: Some("Specify the exact subdirectory you want to remove".to_string()),
            },
            SafetyRule {
                pattern: r"chmod\s+(-[a-zA-Z]*R[a-zA-Z]*\s+)?777\s+/\s*$".to_string(),
                level: RiskLevel::Critical,
                category: "filesystem".to_string(),
                message: "Setting world-writable permissions on the root filesystem".to_string(),
                suggestion: Some("Use more restrictive permissions (e.g., 755) and target specific directories".to_string()),
            },
            SafetyRule {
                pattern: r"dd\s+.*if=/dev/zero\s+.*of=/dev/sd".to_string(),
                level: RiskLevel::Critical,
                category: "filesystem".to_string(),
                message: "Writing zeros to a disk device will destroy all data".to_string(),
                suggestion: Some("Double-check the target device. Use lsblk to verify".to_string()),
            },
            SafetyRule {
                pattern: r"\bmkfs\b".to_string(),
                level: RiskLevel::Critical,
                category: "filesystem".to_string(),
                message: "Creating a filesystem will destroy all data on the target device".to_string(),
                suggestion: Some("Verify the target device with lsblk before formatting".to_string()),
            },
            SafetyRule {
                pattern: r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:".to_string(),
                level: RiskLevel::Critical,
                category: "system".to_string(),
                message: "Fork bomb detected - this will exhaust system resources and crash the system".to_string(),
                suggestion: Some("Do not run fork bombs. If testing, use a VM or container with resource limits".to_string()),
            },
            SafetyRule {
                pattern: r"curl\s+.*\|\s*(sudo\s+)?(ba)?sh".to_string(),
                level: RiskLevel::Critical,
                category: "security".to_string(),
                message: "Piping curl output directly to a shell is dangerous - executes unreviewed code".to_string(),
                suggestion: Some("Download the script first, review it, then execute: curl -o script.sh <url> && cat script.sh && bash script.sh".to_string()),
            },
            SafetyRule {
                pattern: r"wget\s+.*\|\s*(sudo\s+)?(ba)?sh".to_string(),
                level: RiskLevel::Critical,
                category: "security".to_string(),
                message: "Piping wget output directly to a shell is dangerous - executes unreviewed code".to_string(),
                suggestion: Some("Download the script first, review it, then execute".to_string()),
            },
        ]
    }

    fn high_rules() -> Vec<SafetyRule> {
        vec![
            SafetyRule {
                pattern: r"git\s+push\s+.*--force\b".to_string(),
                level: RiskLevel::High,
                category: "git".to_string(),
                message: "Force pushing can overwrite remote history and cause data loss for collaborators".to_string(),
                suggestion: Some("Use --force-with-lease instead for safer force pushes".to_string()),
            },
            SafetyRule {
                pattern: r"git\s+push\s+.*--force\b.*\b(main|master|develop)\b".to_string(),
                level: RiskLevel::High,
                category: "git".to_string(),
                message: "Force pushing to a protected branch (main/master/develop) can cause severe data loss".to_string(),
                suggestion: Some("Never force push to shared branches. Create a new commit to fix issues instead".to_string()),
            },
            SafetyRule {
                pattern: r"git\s+reset\s+--hard\b".to_string(),
                level: RiskLevel::High,
                category: "git".to_string(),
                message: "Hard reset discards all uncommitted changes permanently".to_string(),
                suggestion: Some("Use git stash to save changes first, or use --soft/--mixed reset".to_string()),
            },
            SafetyRule {
                pattern: r"git\s+clean\s+(-[a-zA-Z]*f[a-zA-Z]*\s+(-[a-zA-Z]*d[a-zA-Z]*)?|(-[a-zA-Z]*d[a-zA-Z]*\s+)?-[a-zA-Z]*f[a-zA-Z]*)".to_string(),
                level: RiskLevel::High,
                category: "git".to_string(),
                message: "git clean -fd permanently removes untracked files and directories".to_string(),
                suggestion: Some("Run git clean -n first to preview what will be deleted".to_string()),
            },
            SafetyRule {
                pattern: r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|(-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+).*\.git\b".to_string(),
                level: RiskLevel::High,
                category: "git".to_string(),
                message: "Removing a directory containing .git will destroy the repository history".to_string(),
                suggestion: Some("Make sure you have a remote backup before removing a git repository".to_string()),
            },
            SafetyRule {
                pattern: r"(?i)\bDROP\s+DATABASE\b".to_string(),
                level: RiskLevel::High,
                category: "database".to_string(),
                message: "DROP DATABASE permanently destroys an entire database".to_string(),
                suggestion: Some("Create a backup first: pg_dump or mysqldump".to_string()),
            },
            SafetyRule {
                pattern: r"(?i)\bDROP\s+TABLE\b".to_string(),
                level: RiskLevel::High,
                category: "database".to_string(),
                message: "DROP TABLE permanently destroys a table and all its data".to_string(),
                suggestion: Some("Create a backup of the table first".to_string()),
            },
            SafetyRule {
                pattern: r"(?i)\bTRUNCATE\b".to_string(),
                level: RiskLevel::High,
                category: "database".to_string(),
                message: "TRUNCATE removes all rows from a table without logging individual deletions".to_string(),
                suggestion: Some("Use DELETE with a WHERE clause for safer data removal, or backup first".to_string()),
            },
            SafetyRule {
                pattern: r"docker\s+system\s+prune\s+(-[a-zA-Z]*a|--all)".to_string(),
                level: RiskLevel::High,
                category: "docker".to_string(),
                message: "docker system prune -a removes ALL unused images, containers, networks, and volumes".to_string(),
                suggestion: Some("Use docker system prune (without -a) to only remove dangling resources, or prune specific resource types".to_string()),
            },
            SafetyRule {
                pattern: r"kubectl\s+delete\s+namespace\b".to_string(),
                level: RiskLevel::High,
                category: "kubernetes".to_string(),
                message: "Deleting a Kubernetes namespace removes all resources within it".to_string(),
                suggestion: Some("List resources first with kubectl get all -n <namespace> before deleting".to_string()),
            },
            SafetyRule {
                pattern: r"terraform\s+destroy\b".to_string(),
                level: RiskLevel::High,
                category: "infrastructure".to_string(),
                message: "terraform destroy will tear down all managed infrastructure".to_string(),
                suggestion: Some("Run terraform plan -destroy first to preview what will be destroyed".to_string()),
            },
            SafetyRule {
                pattern: r"npm\s+publish\b".to_string(),
                level: RiskLevel::High,
                category: "package".to_string(),
                message: "Publishing to npm is a public and largely irreversible action".to_string(),
                suggestion: Some("Use npm publish --dry-run first to verify what will be published".to_string()),
            },
        ]
    }

    fn medium_rules() -> Vec<SafetyRule> {
        vec![
            SafetyRule {
                pattern: r"rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|(-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+)node_modules\b".to_string(),
                level: RiskLevel::Medium,
                category: "filesystem".to_string(),
                message: "Removing node_modules recursively - this can take a while and may cause issues if done in the wrong directory".to_string(),
                suggestion: Some("Use the project's package manager to clean: npm ci or yarn install --frozen-lockfile".to_string()),
            },
            SafetyRule {
                pattern: r"git\s+checkout\s+--\s+\.".to_string(),
                level: RiskLevel::Medium,
                category: "git".to_string(),
                message: "git checkout -- . discards all unstaged changes in the working directory".to_string(),
                suggestion: Some("Use git stash to save changes, or git diff to review before discarding".to_string()),
            },
            SafetyRule {
                pattern: r"chmod\s+(-[a-zA-Z]*R[a-zA-Z]*)\s+".to_string(),
                level: RiskLevel::Medium,
                category: "filesystem".to_string(),
                message: "Recursive chmod can change permissions on many files at once".to_string(),
                suggestion: Some("Verify the target directory and use find with -type to apply permissions selectively".to_string()),
            },
            SafetyRule {
                pattern: r"find\s+/\s+.*-delete\b".to_string(),
                level: RiskLevel::Medium,
                category: "filesystem".to_string(),
                message: "Recursive find with -delete from root can remove critical files".to_string(),
                suggestion: Some("Run without -delete first to preview matches, and use a more specific starting path".to_string()),
            },
            SafetyRule {
                pattern: r"chown\s+(-[a-zA-Z]*R[a-zA-Z]*)\s+".to_string(),
                level: RiskLevel::Medium,
                category: "filesystem".to_string(),
                message: "Recursive chown can change ownership on many files at once".to_string(),
                suggestion: Some("Verify the target directory and consider using find with -exec for more control".to_string()),
            },
            SafetyRule {
                pattern: r"rsync\s+.*--delete\b".to_string(),
                level: RiskLevel::Medium,
                category: "filesystem".to_string(),
                message: "rsync with --delete will remove files in the destination that are not in the source".to_string(),
                suggestion: Some("Use --dry-run first to preview the changes".to_string()),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_all_rules_have_valid_regex() {
        for rule in BuiltinRules::all() {
            assert!(
                Regex::new(&rule.pattern).is_ok(),
                "Invalid regex pattern in rule '{}': {}",
                rule.message,
                rule.pattern
            );
        }
    }

    #[test]
    fn test_all_rules_returned() {
        let rules = BuiltinRules::all();
        assert!(!rules.is_empty());

        let critical_count = rules
            .iter()
            .filter(|r| r.level == RiskLevel::Critical)
            .count();
        let high_count = rules.iter().filter(|r| r.level == RiskLevel::High).count();
        let medium_count = rules
            .iter()
            .filter(|r| r.level == RiskLevel::Medium)
            .count();

        assert!(
            critical_count >= 10,
            "Expected at least 10 critical rules, got {critical_count}"
        );
        assert!(
            high_count >= 12,
            "Expected at least 12 high rules, got {high_count}"
        );
        assert!(
            medium_count >= 6,
            "Expected at least 6 medium rules, got {medium_count}"
        );
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(RiskLevel::Critical.to_string(), "Critical");
        assert_eq!(RiskLevel::High.to_string(), "High");
        assert_eq!(RiskLevel::Medium.to_string(), "Medium");
        assert_eq!(RiskLevel::Low.to_string(), "Low");
    }

    #[test]
    fn test_critical_patterns_match() {
        let rules = BuiltinRules::all();
        let critical_rules: Vec<_> = rules
            .iter()
            .filter(|r| r.level == RiskLevel::Critical)
            .collect();

        // rm -rf /
        let re = Regex::new(&critical_rules[0].pattern).unwrap();
        assert!(re.is_match("rm -rf /"));
        assert!(re.is_match("rm -r -f /"));

        // Fork bomb
        let fork_rule = critical_rules
            .iter()
            .find(|r| r.category == "system")
            .unwrap();
        let re = Regex::new(&fork_rule.pattern).unwrap();
        assert!(re.is_match(":(){ :|:& };:"));

        // curl | sh
        let curl_rule = critical_rules
            .iter()
            .find(|r| r.message.contains("curl"))
            .unwrap();
        let re = Regex::new(&curl_rule.pattern).unwrap();
        assert!(re.is_match("curl https://example.com/install.sh | sh"));
        assert!(re.is_match("curl https://example.com/install.sh | bash"));
        assert!(re.is_match("curl https://example.com/install.sh | sudo sh"));
    }
}
