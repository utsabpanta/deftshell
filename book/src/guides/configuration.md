# Configuration

## Overview

DeftShell uses TOML configuration with a hierarchical loading order:

1. **Built-in defaults** — Sensible defaults for all settings
2. **Global config** — `~/.deftshell/config.toml`
3. **Project config** — `.deftshell.toml` in the project root
4. **Environment variables** — `DS_*` prefix
5. **CLI flags** — Per-command overrides

Each level overrides the previous. Project config is committed to your repo so team members share the same settings.

## Configuration Commands

```bash
ds config                    # Open config in $EDITOR
ds config get <key>          # Get a value
ds config set <key> <value>  # Set a value
ds config reset              # Reset to defaults
ds config validate           # Validate current config
ds config path               # Show config file path
ds config export             # Export as JSON
```

## Global Configuration

The global config lives at `~/.deftshell/config.toml`:

```toml
[general]
telemetry = false
update_check = true
update_channel = "stable"    # "stable" | "beta" | "nightly"
log_level = "warn"           # "trace" | "debug" | "info" | "warn" | "error"

[shell]
default = "zsh"              # "zsh" | "bash" | "fish"
integration_mode = "full"    # "full" | "prompt-only" | "passive"

[prompt]
theme = "default"            # "default" | "minimal" | "powerline" | "pure"
show_git = true
show_stack = true
show_env = true
show_execution_time = true
execution_time_threshold_ms = 2000
transient_prompt = true
right_prompt = true

[ai]
default_provider = "ollama"
fallback_provider = "openai"
privacy_mode = false
privacy_mode_provider = "ollama"

[ai.providers.anthropic]
enabled = true
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"
max_tokens = 4096

[ai.providers.openai]
enabled = true
api_key_env = "OPENAI_API_KEY"
model = "gpt-4o"

[ai.providers.ollama]
enabled = true
host = "http://localhost:11434"
model = "llama3.1"

[safety]
enabled = true
confirm_threshold = "medium"  # "low" | "medium" | "high" | "critical"
require_confirmation = true

[analytics]
enabled = true
retention_days = 365

[plugins]
auto_update = true
```

## Project Configuration

Create `.deftshell.toml` in your project root:

```toml
[project]
name = "my-app"
team = "platform"

[scripts]
setup = "pnpm install && pnpm prisma generate"
dev = "pnpm dev"
test = "pnpm vitest"

[ai.context]
include_files = ["docs/architecture.md"]
exclude_files = [".env*", "secrets/"]

[[safety.custom_rules.rule]]
pattern = "prisma migrate reset"
level = "high"
message = "This resets the database. Are you sure?"

[aliases]
dev = "pnpm dev"
test-watch = "pnpm vitest --watch"
```

## Credential Storage

API keys are stored in `~/.deftshell/credentials.toml` with restrictive file permissions (0600 on Unix):

```toml
[auth]
anthropic_api_key = "sk-ant-..."
openai_api_key = "sk-..."
```

Use `ds auth <provider>` to set up credentials interactively.
