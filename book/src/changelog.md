# Changelog

All notable changes to DeftShell will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-05

### Added

- Initial release of DeftShell
- Shell integration for Zsh, Bash, and Fish with precmd/preexec/chpwd hooks
- Context detection engine: auto-detects language, framework, package manager, Docker, CI/CD, cloud provider, services
- Monorepo workspace detection (npm, pnpm, yarn, cargo, turborepo, nx, lerna)
- AI gateway with multi-provider support: Anthropic Claude, OpenAI, Google Gemini, Ollama, GitHub Copilot, AWS Bedrock
- Automatic provider fallback and privacy mode (local-only AI via Ollama)
- Safety engine with 20+ built-in rules for dangerous command interception
- Context-aware risk elevation (production environments, protected branches, uncommitted changes)
- Command intelligence: typo correction, alias suggestions, command sequence detection
- Command history tracking with sensitive data redaction
- Analytics engine with usage statistics and TUI dashboard
- Runbook system: create, edit, record, execute, and share multi-step workflows
- Runbook variables, conditions, and AI-powered generation
- Plugin system with npm-based distribution
- Smart prompt with git status, stack info, execution time, and AI status
- TOML configuration with hierarchical loading (global, project, env, CLI)
- SQLite storage with context caching and AI usage tracking
- Keychain integration for secure credential storage
- Shell completions for Zsh, Bash, and Fish
- CI/CD pipelines for GitHub Actions
- Comprehensive test suite (84+ tests)
