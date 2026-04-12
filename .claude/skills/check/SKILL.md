---
name: check
description: "Run the full CI check locally: fmt, clippy, tests"
allowed-tools: "Bash(cargo *)"
---

# Full CI Check

Run these in order. Stop and report on first failure.

1. Format check: `cargo fmt --all -- --check`
2. Lint: `cargo clippy --workspace -- -D warnings`
3. Test: `cargo test --workspace`
4. Report: pass/fail for each step, total test count, and any warnings.
