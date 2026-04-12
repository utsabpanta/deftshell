# AI Providers

DeftShell supports multiple AI providers with automatic fallback. Configure them in `~/.deftshell/config.toml`.

## Supported Providers

| Provider | Type | Best For |
|----------|------|----------|
| Anthropic Claude | Cloud | Best overall quality |
| OpenAI GPT | Cloud | Wide model selection |
| Google Gemini | Cloud | Large context windows |
| Ollama | Local | Privacy, no API costs |
| GitHub Copilot | Cloud | Existing Copilot users |
| AWS Bedrock | Cloud | Enterprise/AWS environments |

## GitHub Copilot

Easiest setup if you already have GitHub CLI:

```bash
# If you have GitHub CLI installed and logged in:
ds auth copilot                            # Auto-detects gh token, sets as default
ds ask "hello"                             # Just works!

# Not logged in yet?
gh auth login                              # One-time: log in to GitHub
ds auth copilot                            # Sets Copilot as default
```

Uses the GitHub Models API with your GitHub token. No extra scopes or manual config needed.

```toml
[ai.providers.copilot]
enabled = true
```

## Anthropic Claude

```bash
ds auth anthropic                          # Prompts for API key, stores securely
ds config set ai.default_provider anthropic
# Or set the env var: export ANTHROPIC_API_KEY=sk-ant-...
```

```toml
[ai.providers.anthropic]
enabled = true
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"  # or claude-opus-4-20250514
max_tokens = 4096
```

## OpenAI

```bash
ds auth openai                             # Prompts for API key
ds config set ai.default_provider openai
# Or: export OPENAI_API_KEY=sk-...
```

```toml
[ai.providers.openai]
enabled = true
api_key_env = "OPENAI_API_KEY"
model = "gpt-4o"
```

## Ollama (Local)

Free, private, runs entirely on your machine:

```bash
# Install: https://ollama.com
ollama serve                               # Start the server
ollama pull llama3.1                       # Download a model
ds config set ai.default_provider ollama
```

```toml
[ai.providers.ollama]
enabled = true
host = "http://localhost:11434"
model = "llama3.1"
```

Ollama is the default privacy-mode provider — when privacy mode is on, all AI queries go through Ollama.

## Google Gemini

```bash
ds auth gemini                             # Prompts for API key
ds config set ai.default_provider gemini
# Or: export GEMINI_API_KEY=...
```

```toml
[ai.providers.gemini]
enabled = true
api_key_env = "GEMINI_API_KEY"
model = "gemini-pro"
```

## AWS Bedrock

For enterprise/AWS environments:

```bash
ds auth bedrock                            # Prompts for AWS profile name
ds config set ai.default_provider bedrock
# Requires: aws configure --profile <name>
```

```toml
[ai.providers.bedrock]
enabled = true
region = "us-east-1"
model_id = "anthropic.claude-3-sonnet-20240229-v1:0"
aws_profile = "default"  # optional
```

Requires AWS credentials:

```bash
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"
```

## Fallback Configuration

Set a fallback provider for when the primary is unavailable:

```toml
[ai]
default_provider = "anthropic"
fallback_provider = "ollama"
```

## Per-Command Override

Use any provider for a single command without changing defaults:

```bash
ds ask "hello" --provider copilot
ds ask "hello" --provider ollama
```

## Usage Tracking

Monitor your AI usage:

```bash
ds usage    # Show token usage and costs
```
