# Config Reference

Complete TOML configuration schema for DeftShell.

## `[general]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `telemetry` | bool | `false` | Enable anonymous usage telemetry |
| `update_check` | bool | `true` | Check for updates on startup |
| `update_channel` | string | `"stable"` | `"stable"`, `"beta"`, or `"nightly"` |
| `log_level` | string | `"warn"` | `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"` |

## `[shell]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default` | string | `"zsh"` | Default shell: `"zsh"`, `"bash"`, `"fish"` |
| `integration_mode` | string | `"full"` | `"full"`, `"prompt-only"`, `"passive"` |

## `[prompt]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"default"` | `"default"`, `"minimal"`, `"powerline"`, `"pure"` |
| `show_git` | bool | `true` | Show git branch and status |
| `show_stack` | bool | `true` | Show detected framework |
| `show_env` | bool | `true` | Show environment (dev/staging/prod) |
| `show_execution_time` | bool | `true` | Show last command duration |
| `execution_time_threshold_ms` | int | `2000` | Min duration (ms) to display |
| `transient_prompt` | bool | `true` | Collapse previous prompts |
| `right_prompt` | bool | `true` | Show right-aligned info |

## `[ai]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_provider` | string | `"ollama"` | Default AI provider |
| `fallback_provider` | string | `"openai"` | Fallback when primary fails |
| `privacy_mode` | bool | `false` | Route all AI through local provider |
| `privacy_mode_provider` | string | `"ollama"` | Provider for privacy mode |

## `[ai.providers.anthropic]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable this provider |
| `api_key_env` | string | `"ANTHROPIC_API_KEY"` | Env var for API key |
| `model` | string | `"claude-sonnet-4-20250514"` | Model to use |
| `max_tokens` | int | `4096` | Max tokens per response |

## `[ai.providers.openai]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable this provider |
| `api_key_env` | string | `"OPENAI_API_KEY"` | Env var for API key |
| `model` | string | `"gpt-4o"` | Model to use |

## `[ai.providers.ollama]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable this provider |
| `host` | string | `"http://localhost:11434"` | Ollama server URL |
| `model` | string | `"llama3.1"` | Model to use |

## `[ai.providers.gemini]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable this provider |
| `api_key_env` | string | `"GEMINI_API_KEY"` | Env var for API key |
| `model` | string | `"gemini-pro"` | Model to use |

## `[ai.providers.copilot]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable this provider |

Token is auto-detected from `~/.config/github-copilot/hosts.json`, `gh auth token`, or `GITHUB_TOKEN`.

## `[ai.providers.bedrock]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable this provider |
| `region` | string | `"us-east-1"` | AWS region |
| `model_id` | string | `"anthropic.claude-3-sonnet-20240229-v1:0"` | Model ID |
| `aws_profile` | string | `"default"` | AWS profile name (optional) |

## `[safety]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable safety engine |
| `confirm_threshold` | string | `"medium"` | `"low"`, `"medium"`, `"high"`, `"critical"` |
| `require_confirmation` | bool | `true` | Require explicit yes/no |
| `allowlist` | string[] | `[]` | Commands that bypass safety |
| `denylist` | string[] | `[]` | Commands always blocked |

## `[safety.custom_rules]`

```toml
[[safety.custom_rules.rule]]
pattern = "prisma migrate reset"
level = "high"
message = "This will reset the database."
suggestion = "Use 'prisma migrate dev' instead"
```

## `[analytics]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable analytics tracking |
| `retention_days` | int | `365` | Days to retain analytics data |

## `[plugins]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `auto_update` | bool | `true` | Auto-update plugins |

## Project Config (`[project]`)

These fields go in `.deftshell.toml` at your project root:

| Key | Type | Description |
|-----|------|-------------|
| `name` | string | Project name |
| `team` | string | Team name |

## Project Config (`[ai.context]`)

| Key | Type | Description |
|-----|------|-------------|
| `include_files` | string[] | Files to include in AI context |
| `exclude_files` | string[] | Glob patterns to exclude from AI context |

## Project Config (`[scripts]`)

```toml
[scripts]
setup = "pnpm install && pnpm prisma generate"
dev = "pnpm dev"
test = "pnpm vitest"
```

## Project Config (`[aliases]`)

```toml
[aliases]
dev = "pnpm dev"
test-watch = "pnpm vitest --watch"
```
