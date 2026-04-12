# AI Provider Setup

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

## Anthropic Claude

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

```toml
[ai]
default_provider = "anthropic"

[ai.providers.anthropic]
enabled = true
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"  # or claude-opus-4-20250514
max_tokens = 4096
```

## OpenAI

```bash
export OPENAI_API_KEY="sk-..."
```

```toml
[ai.providers.openai]
enabled = true
api_key_env = "OPENAI_API_KEY"
model = "gpt-4o"
```

## Ollama (Local)

Install from [ollama.ai](https://ollama.ai), then:

```bash
ollama pull llama3.1
```

```toml
[ai.providers.ollama]
enabled = true
host = "http://localhost:11434"
model = "llama3.1"
```

Ollama is the default privacy-mode provider — when privacy mode is on, all AI queries go through Ollama instead of cloud providers.

## Google Gemini

```bash
export GEMINI_API_KEY="..."
```

```toml
[ai.providers.gemini]
enabled = true
api_key_env = "GEMINI_API_KEY"
model = "gemini-pro"
```

## GitHub Copilot

Uses your existing GitHub Copilot subscription. Token is read from `~/.config/github-copilot/hosts.json` or the environment:

```bash
export GITHUB_COPILOT_TOKEN="..."
```

```toml
[ai.providers.copilot]
enabled = true
```

## AWS Bedrock

Requires AWS credentials:

```bash
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"
```

```toml
[ai.providers.bedrock]
enabled = true
region = "us-east-1"
model_id = "anthropic.claude-3-sonnet-20240229-v1:0"
aws_profile = "default"  # optional
```

## Fallback Configuration

Set a fallback provider for when the primary is unavailable:

```toml
[ai]
default_provider = "anthropic"
fallback_provider = "ollama"
```

## Privacy Mode

Route all AI queries through a local provider:

```bash
ds privacy on   # Enable privacy mode
ds privacy off  # Disable privacy mode
```

```toml
[ai]
privacy_mode = false
privacy_mode_provider = "ollama"
```

## Usage Tracking

Monitor your AI usage:

```bash
ds usage    # Show token usage and costs
ds stats ai # AI usage analytics
```
