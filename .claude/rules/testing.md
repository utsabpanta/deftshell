---
paths:
  - "crates/**/*.rs"
  - "tests/**/*.rs"
---

# Testing Conventions

- Unit tests go inside the module file in a `#[cfg(test)] mod tests` block
- Integration tests for ds-core go in `crates/ds-core/tests/integration.rs`
- CLI integration tests go in `crates/ds-cli/tests/cli_integration.rs`
- CLI tests use `assert_cmd` and `predicates` crates to invoke the compiled binary
- Use `tempfile::TempDir` for any test that touches the filesystem
- Database tests should use `Database::open_in_memory()` not a real file
- Test the happy path, one error case, and one edge case at minimum
- Run `cargo test --workspace` to verify all tests pass
- Safety rule tests must cover: pattern matches, pattern doesn't false-positive on safe input
- Context detection tests use fixtures in `tests/fixtures/`
