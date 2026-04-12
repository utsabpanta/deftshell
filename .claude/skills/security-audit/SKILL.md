---
name: security-audit
description: "Run a security audit on the DeftShell codebase"
allowed-tools: "Bash(cargo *) Read Grep Glob"
---

# DeftShell Security Audit

Perform a thorough security review of the codebase. Check each area and report findings with severity levels (CRITICAL, HIGH, MEDIUM, LOW).

## 1. Credential Handling
- Check `crates/ds-core/src/storage/keychain.rs` for proper file permissions
- Verify no API keys appear in log output or error messages
- Check all AI providers for hardcoded keys or keys in URLs

## 2. Command Injection
- Check `crates/ds-core/src/runbook/executor.rs` for shell injection via variables
- Check `crates/ds-cli/src/commands/do_cmd.rs` for AI-generated command validation
- Check `crates/ds-cli/src/commands/chat.rs` for code block extraction safety

## 3. SQL Injection
- Verify all queries in `crates/ds-core/src/storage/db.rs` use parameterized statements

## 4. Path Traversal
- Check plugin loader for path validation on plugin names
- Check any user-provided file paths

## 5. Input Validation
- Search for `unwrap()` and `expect()` in non-test code
- Check for unbounded `read_to_string()` calls without size limits

## 6. Dependencies
- Run `cargo audit` if available, otherwise check Cargo.lock for known issues

## 7. Report
Produce a summary table of findings with severity, location, and recommended fix.
