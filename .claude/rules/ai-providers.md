---
paths:
  - "crates/ds-core/src/ai/**/*.rs"
---

# AI Provider Guidelines

- All providers implement the `AiProvider` async trait: `name()`, `is_available()`, `complete()`, `stream()`
- Provider constructors must set HTTP timeout via `Client::builder().timeout(...).build()`
- API keys resolve via `resolve_api_key()`: env var first, then credential store
- Non-streaming responses must extract real token counts from the API response
- Streaming responses return `StreamChunk` with content only (token counts estimated downstream)
- Cost tracking is not yet implemented — pass 0.0 for cost parameter
- The gateway handles fallback: primary provider fails → try fallback_provider
- Privacy mode routes all requests through the local provider (Ollama)
- New providers must be registered in `gateway.rs` `register_default_providers()`
- Provider tests should verify: endpoint URL construction, request format, response parsing
