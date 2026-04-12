//! CLI integration tests for the `ds` binary.
//!
//! These tests invoke the compiled binary with `assert_cmd` and verify
//! exit codes, stdout, and stderr for every major subcommand that can
//! run without external dependencies (no AI providers, no network).

use assert_cmd::Command;
use predicates::prelude::*;

/// Helper: build a `Command` pointing at the `ds` binary.
fn ds() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("ds").expect("failed to find ds binary")
}

// =========================================================================
// Version & help
// =========================================================================

#[test]
fn version_flag_prints_version() {
    ds().arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ds"));
}

#[test]
fn version_subcommand_prints_info() {
    ds().arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("DeftShell"));
}

#[test]
fn help_flag_prints_usage() {
    ds().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI-powered").or(predicate::str::contains("AI-Powered")));
}

#[test]
fn help_subcommand_no_args_prints_overview() {
    ds().arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("ds"));
}

// =========================================================================
// Shell init scripts
// =========================================================================

#[test]
fn init_zsh_outputs_script() {
    ds().args(["init", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_deftshell_precmd"))
        .stdout(predicate::str::contains("DEFTSHELL_SHELL"));
}

#[test]
fn init_bash_outputs_script() {
    ds().args(["init", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_deftshell_prompt_command"))
        .stdout(predicate::str::contains("DEFTSHELL_SHELL"));
}

#[test]
fn init_fish_outputs_script() {
    ds().args(["init", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fish_prompt"))
        .stdout(predicate::str::contains("DEFTSHELL_SHELL"));
}

#[test]
fn init_invalid_shell_fails() {
    ds().args(["init", "powershell"]).assert().failure().stderr(
        predicate::str::contains("Unknown shell").or(predicate::str::contains("Unsupported")),
    );
}

// =========================================================================
// Shell completions
// =========================================================================

#[test]
fn completions_zsh() {
    ds().args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn completions_bash() {
    ds().args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn completions_fish() {
    ds().args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// =========================================================================
// Context detection
// =========================================================================

#[test]
fn context_detects_project() {
    // Run in the deftshell project root — should detect Rust.
    ds().arg("context")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust").or(predicate::str::contains("Rust")));
}

#[test]
fn context_export_json() {
    ds().args(["context", "export"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
fn context_detect_quiet_succeeds() {
    ds().args(["context", "--detect", "--quiet"])
        .assert()
        .success();
}

// =========================================================================
// Safety check
// =========================================================================

#[test]
fn safety_check_dangerous_command_exits_nonzero() {
    ds().args(["safety-check", "rm -rf /"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("CRITICAL").or(predicate::str::contains("WARNING")));
}

#[test]
fn safety_check_safe_command_exits_zero() {
    ds().args(["safety-check", "ls -la"]).assert().success();
}

#[test]
fn safety_check_git_force_push() {
    ds().args(["safety-check", "git push --force origin main"])
        .assert()
        .failure();
}

#[test]
fn safety_check_chmod_recursive() {
    ds().args(["safety-check", "chmod -R 777 /"])
        .assert()
        .failure();
}

// =========================================================================
// Config
// =========================================================================

#[test]
fn config_path_prints_path() {
    ds().args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn config_validate_succeeds() {
    ds().args(["config", "validate"]).assert().success();
}

#[test]
fn config_export_outputs_json() {
    ds().args(["config", "export"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
fn config_get_known_key() {
    ds().args(["config", "get", "ai.default_provider"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn config_get_unknown_key() {
    ds().args(["config", "get", "nonexistent.key.path"])
        .assert()
        .failure();
}

// =========================================================================
// Auth
// =========================================================================

#[test]
fn auth_status_lists_providers() {
    ds().args(["auth", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("anthropic"))
        .stdout(predicate::str::contains("openai"))
        .stdout(predicate::str::contains("copilot"));
}

// =========================================================================
// AI commands — input validation (no network calls)
// =========================================================================

#[test]
fn ask_empty_query_fails() {
    ds().args(["ask", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("provide a question"));
}

#[test]
fn do_empty_instruction_fails() {
    ds().args(["do", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("provide an instruction"));
}

#[test]
fn explain_no_stdin_fails() {
    ds().arg("explain")
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No input provided"));
}

#[test]
fn review_no_stdin_fails() {
    ds().arg("review")
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No input provided"));
}

// =========================================================================
// Stats
// =========================================================================

#[test]
fn stats_runs_without_error() {
    ds().arg("stats").assert().success();
}

#[test]
fn stats_today() {
    ds().args(["stats", "today"]).assert().success();
}

// =========================================================================
// Usage
// =========================================================================

#[test]
fn usage_runs_without_error() {
    ds().arg("usage").assert().success();
}

// =========================================================================
// Doctor
// =========================================================================

#[test]
fn doctor_runs_diagnostics() {
    ds().arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("DeftShell").or(predicate::str::contains("doctor")));
}

// =========================================================================
// Aliases
// =========================================================================

#[test]
fn alias_export_succeeds() {
    ds().args(["alias", "--export"]).assert().success();
}

// =========================================================================
// Env
// =========================================================================

#[test]
fn env_prints_info() {
    ds().arg("env")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// =========================================================================
// Scripts
// =========================================================================

#[test]
fn scripts_command_succeeds() {
    ds().arg("scripts").assert().success();
}

// =========================================================================
// Runbook
// =========================================================================

#[test]
fn runbook_list_succeeds() {
    ds().args(["runbook", "list"]).assert().success();
}

// =========================================================================
// Plugin
// =========================================================================

#[test]
fn plugin_list_succeeds() {
    ds().args(["plugin", "list"]).assert().success();
}

// =========================================================================
// Provider flag (--provider)
// =========================================================================

#[test]
fn provider_flag_is_accepted() {
    // The flag should be accepted even though the query will fail on empty input.
    ds().args(["--provider", "anthropic", "ask", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("provide a question"));
}

// =========================================================================
// Track command (internal)
// =========================================================================

#[test]
fn track_command_succeeds() {
    ds().args([
        "track-command",
        "--command",
        "ls -la",
        "--exit-code",
        "0",
        "--duration",
        "100",
        "--dir",
        "/tmp",
    ])
    .assert()
    .success();
}

// =========================================================================
// Prompt segment (internal)
// =========================================================================

#[test]
fn prompt_segment_zsh_succeeds() {
    ds().args([
        "prompt-segment",
        "--shell",
        "zsh",
        "--exit-code",
        "0",
        "--duration",
        "100",
    ])
    .assert()
    .success()
    .stdout(predicate::str::is_empty().not());
}

#[test]
fn prompt_segment_bash_succeeds() {
    ds().args([
        "prompt-segment",
        "--shell",
        "bash",
        "--exit-code",
        "1",
        "--duration",
        "5000",
    ])
    .assert()
    .success()
    .stdout(predicate::str::is_empty().not());
}

// =========================================================================
// Privacy
// =========================================================================

#[test]
fn privacy_on() {
    ds().args(["privacy", "on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Privacy mode"));
}

#[test]
fn privacy_off() {
    ds().args(["privacy", "off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Privacy mode"));
}
