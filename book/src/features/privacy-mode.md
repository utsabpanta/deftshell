# Privacy Mode

Privacy mode routes all AI queries through a local provider (Ollama by default) so no data leaves your machine.

## Enabling Privacy Mode

```bash
ds privacy on     # Enable privacy mode
ds privacy off    # Disable privacy mode
```

Or set it in your configuration:

```toml
[ai]
privacy_mode = false
privacy_mode_provider = "ollama"
```

## How It Works

When privacy mode is enabled, the AI Gateway overrides your `default_provider` setting and routes all requests through the configured `privacy_mode_provider` (Ollama by default).

This means:
- All AI queries are processed locally on your machine
- No data is sent to cloud AI providers
- You need Ollama (or another local provider) installed and running

## Setting Up Ollama

1. Install Ollama from [ollama.com](https://ollama.com)
2. Start the server: `ollama serve`
3. Download a model: `ollama pull llama3.1`
4. Enable privacy mode: `ds privacy on`

```toml
[ai.providers.ollama]
enabled = true
host = "http://localhost:11434"
model = "llama3.1"
```

## Use Cases

- Working on proprietary/sensitive codebases
- Corporate environments with data residency requirements
- Offline development (no internet required after model download)
- Reducing API costs while still using AI features
