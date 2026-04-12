# Contributing to DeftShell

Thank you for your interest in contributing to DeftShell! This guide will help you get started.

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Node.js 22+ (optional, needed for JavaScript plugins)
- SQLite 3.x (usually pre-installed on macOS/Linux)

### Building from Source

```bash
git clone https://github.com/utsabpanta/deftshell.git
cd deftshell
cargo build
```

### Running Tests

```bash
cargo test --workspace
```

### Project Structure

```
deftshell/
├── crates/
│   ├── ds-cli/          # CLI binary (clap-based)
│   ├── ds-core/         # Core library
│   │   ├── ai/          # AI gateway and providers
│   │   ├── config/      # Configuration loading
│   │   ├── context/     # Project context detection
│   │   ├── intelligence/# Command tracking and suggestions
│   │   ├── runbook/     # Runbook management
│   │   ├── safety/      # Command safety engine
│   │   ├── shell/       # Shell integration and prompt
│   │   └── storage/     # SQLite database and keychain
│   └── ds-plugin-sdk/   # Rust plugin SDK
├── shell/               # Shell integration scripts
├── tests/               # Integration and E2E tests
└── docs/                # Documentation
```

## Code Guidelines

### Rust

- Follow standard Rust conventions (`cargo fmt`, `cargo clippy`)
- Add tests for new functionality
- Keep functions focused and well-documented
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library error types when appropriate

### Commit Messages

Use conventional commit format:

```
feat: add fish shell completion support
fix: correct typo detection for short commands
docs: update AI provider setup guide
test: add context detection edge cases
refactor: simplify runbook variable substitution
```

### Pull Requests

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes with tests
4. Run `cargo test --workspace` and `cargo clippy`
5. Submit a PR with a clear description

## Areas for Contribution

- **AI Providers** — Add support for new AI providers
- **Context Detection** — Improve detection for more frameworks and languages
- **Safety Rules** — Add rules for more dangerous command patterns
- **Plugins** — Build and share community plugins
- **Runbooks** — Create and share useful runbooks
- **Documentation** — Improve guides and API docs
- **Shell Integration** — Enhance support for different shells

## Reporting Issues

Use [GitHub Issues](https://github.com/utsabpanta/deftshell/issues) to report bugs or request features. Include:

- DeftShell version (`ds version`)
- OS and shell type
- Steps to reproduce
- Expected vs actual behavior

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
