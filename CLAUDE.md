# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

DeftShell (`ds`) — an AI-powered context-aware terminal for developers. Rust workspace with 3 crates.

## Build & Development Commands

```bash
cargo build --workspace              # Debug build
cargo build --release                # Release build (LTO, stripped)
cargo test --workspace               # Run all tests
cargo test -p ds-core                # Test a single crate
cargo test -p ds-core context        # Run tests matching "context" in ds-core
cargo clippy --workspace -- -D warnings  # Lint (CI treats warnings as errors)
cargo fmt --all -- --check           # Check formatting
cargo fmt --all                      # Auto-format
```

## Workspace Structure

- **`crates/ds-cli`** — CLI binary. Entry point in `main.rs` uses clap derive macros. Command handlers in `commands/` subdirectory. TUI components in `tui/` (ratatui).
- **`crates/ds-core`** — Core library. All domain logic lives here across 11 modules (see Architecture below).
- **`crates/ds-plugin-sdk`** — Minimal plugin SDK defining `DeftShellPlugin`, `PluginCommand`, `PluginContext` traits.

Dependencies are centralized in the root `Cargo.toml` under `[workspace.dependencies]`.

## Architecture

### AI System (`ds-core/src/ai/`)
`AiGateway` selects a provider and handles fallback. Six providers implement the `AiProvider` async trait (`complete()` + `stream()`): Anthropic, OpenAI, Ollama, Gemini, GitHub Copilot, AWS Bedrock. API keys resolve from env vars first, then keychain via `resolve_api_key()`. `AiContextBuilder` constructs system prompts enriched with detected project context.

### Context Detection (`ds-core/src/context/`)
`ContextDetector::detect()` runs a 12-stage pipeline (package manifests, lock files, config files, monorepo markers, git via `git2`, CI/CD, cloud provider, Docker, etc.) producing a `StackProfile`. Results are cached in SQLite keyed by directory with file-mtime invalidation.

### Safety Engine (`ds-core/src/safety/`)
Commands are checked by `CommandInterceptor` against `BuiltinRules` and custom rules. The safety engine is advisory (warns, does not block execution). `RiskAssessor` elevates risk based on context (git branch, production env, k8s context). Risk levels: Low/Medium/High/Critical.

### Shell Integration (`ds-core/src/shell/`)
Generates init scripts for zsh/bash/fish using shell-specific hooks (`preexec`/`precmd`/`chpwd`). The shell hooks call hidden CLI subcommands (`safety-check`, `track-command`, `prompt-segment`) for interception, tracking, and prompt rendering.

### Storage (`ds-core/src/storage/`)
SQLite database at `~/.deftshell/db.sqlite`. Tables: `command_history`, `context_cache`, `ai_usage` (token/cost tracking), `settings`. `KeychainStore` handles secure API key storage. Schema changes go through `migrations.rs`.

### Other Core Modules
- **`intelligence/`** — Command tracking (`CommandTracker`), analytics (`AnalyticsEngine`), typo/alias suggestions (`SuggestionEngine`)
- **`runbook/`** — YAML/TOML workflow definitions; parse, register, execute with variable substitution, record command sequences
- **`config/`** — Hierarchical config loading: global `~/.deftshell/config.toml` → project `.deftshell.toml` → env vars → CLI flags
- **`plugin/`** — Plugin registry, loader (npm or local), Node.js subprocess runtime

## Conventions

- Error handling: `anyhow::Result` in application code, `thiserror` for library error types
- Git operations use `git2` crate (no subshells)
- Commit messages follow conventional commits (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`)
- Rust edition 2021, minimum Rust 1.75+
