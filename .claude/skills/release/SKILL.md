---
name: release
description: "Run the release checklist for a new DeftShell version"
allowed-tools: "Bash(cargo *) Bash(git *) Bash(ls *) Bash(./target/*) Read Grep"
---

# DeftShell Release Checklist

Run through these steps in order. Stop and report if any step fails.

1. Verify the current branch is `main`: `git branch --show-current`
2. Pull latest: `git pull origin main`
3. Run the full test suite: `cargo test --workspace`
4. Run the linter: `cargo clippy --workspace -- -D warnings`
5. Check formatting: `cargo fmt --all -- --check`
6. Build the release binary: `cargo build --release`
7. Check binary size: `ls -lh target/release/ds`
8. Run the binary to verify it works: `./target/release/ds version`
9. Run `./target/release/ds doctor` to verify all systems
10. Read CHANGELOG.md and check it covers changes since the last git tag
11. Read the version from the root `Cargo.toml` `[workspace.package]` section
12. If a version bump is needed, edit `Cargo.toml` and run `cargo build` to update `Cargo.lock`
13. Create a git tag using the version from Cargo.toml: `git tag -a v<VERSION> -m "Release v<VERSION>"`
14. Report: version number, binary size, test count, and any issues found. Do NOT push — let the user decide.
