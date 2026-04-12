# Environment Variables

All environment variables recognized by DeftShell.

## DeftShell Variables

| Variable | Description |
|----------|-------------|
| `DS_LOG_LEVEL` | Override log level (`trace`, `debug`, `info`, `warn`, `error`) |

## AI Provider Keys

| Variable | Provider | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic | Claude API key |
| `OPENAI_API_KEY` | OpenAI | OpenAI API key |
| `GEMINI_API_KEY` | Gemini | Google Gemini API key |
| `GITHUB_COPILOT_TOKEN` | Copilot | GitHub Copilot token |
| `GITHUB_TOKEN` | Copilot | GitHub token (fallback for Copilot) |

## AWS Bedrock

| Variable | Description |
|----------|-------------|
| `AWS_ACCESS_KEY_ID` | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key |
| `AWS_REGION` | AWS region (e.g., `us-east-1`) |
| `AWS_PROFILE` | AWS profile name |

## Credential Resolution

DeftShell resolves AI provider credentials in this order:

1. **Environment variable** — e.g., `ANTHROPIC_API_KEY`
2. **Credential store** — `~/.deftshell/credentials.toml`
3. **Provider-specific** — e.g., GitHub Copilot reads from `~/.config/github-copilot/hosts.json` or `gh auth token`

The first valid credential found is used. Use `ds auth status` to see which providers have valid credentials.
