---
paths:
  - "crates/**/*.rs"
  - "tests/**/*.rs"
---

# Rust Conventions

- Use `anyhow::Result` for application code error handling
- Use `thiserror` for library error types in ds-core
- Never use `unwrap()` or `expect()` in production code paths — only in tests
- Use `.with_context(|| ...)` to add context to errors, not bare `?`
- All public functions in ds-core must return `Result`
- Use `tracing::debug!` for debug logging, never `println!` in library code
- CLI output: `println!` for data, `eprintln!` for status/progress/errors
- Git operations use `git2` crate, never shell out to `git`
- HTTP clients must have timeouts (90s for remote APIs, 120s for local Ollama)
- Run `cargo clippy --workspace -- -D warnings` before considering code complete
