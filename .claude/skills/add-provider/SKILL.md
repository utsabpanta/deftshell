---
name: add-provider
description: "Scaffold a new AI provider for DeftShell"
allowed-tools: "Read Write Edit Grep Glob Bash(cargo *)"
---

# Add a New AI Provider

Follow these steps to add a new AI provider to DeftShell.

## 1. Create the Provider File
Create `crates/ds-core/src/ai/providers/{name}.rs` implementing the `AiProvider` trait:
- `name()` — return the provider name as a string
- `is_available()` — check if credentials exist (env var or credential store)
- `complete()` — non-streaming API call, return `AiResponse` with real token counts
- `stream()` — streaming API call, return `Stream<Item = Result<StreamChunk>>`

Use `Client::builder().timeout(Duration::from_secs(90)).build()` for the HTTP client.

## 2. Register the Provider
- Add `pub mod {name};` to `crates/ds-core/src/ai/providers/mod.rs`
- Register it in `AiGateway::register_default_providers()` in `gateway.rs`

## 3. Add Config Support
- Add the provider to `AiProviderConfig` if new fields are needed in `config/schema.rs`
- Add auth flow in `crates/ds-cli/src/commands/auth.rs`

## 4. Update Documentation
- Add the provider to the table in README.md under "AI Provider Setup"
- Add auth instructions to GUIDE.md section 9
- Update CLAUDE.md if needed

## 5. Add Tests
- Unit tests for request/response parsing in the provider file
- Add to CLI integration tests in `cli_integration.rs`

## 6. Verify
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
