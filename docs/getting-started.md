# Getting Started with DeftShell

## Installation

### From Source

```bash
cargo install deftshell
```

### Homebrew (macOS)

```bash
brew tap deftshell-io/deftshell
brew install deftshell
```

## Shell Setup

Add shell integration to your shell config:

**Zsh** (`~/.zshrc`):
```bash
eval "$(ds init zsh)"
```

**Bash** (`~/.bashrc`):
```bash
eval "$(ds init bash)"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
ds init fish | source
```

Restart your shell or run `source ~/.zshrc`.

## First Steps

### 1. Check Everything Works

```bash
ds doctor
```

This runs diagnostics and shows the status of all components.

### 2. Set Up an AI Provider

DeftShell works with multiple AI providers. The quickest setup:

**Option A: Ollama (Free, Local)**
```bash
# Install Ollama from https://ollama.ai
ollama pull llama3.1
ds config set ai.default_provider ollama
```

**Option B: Anthropic Claude**
```bash
export ANTHROPIC_API_KEY="your-key-here"
ds config set ai.default_provider anthropic
```

**Option C: OpenAI**
```bash
export OPENAI_API_KEY="your-key-here"
ds config set ai.default_provider openai
```

### 3. Try AI Features

```bash
ds ask "what does this project do?"
ds how "set up a development database"
ds do "find all files larger than 10MB"
```

### 4. Explore Your Project

```bash
ds context       # See what DeftShell detected
ds scripts       # See available project scripts
ds env           # See environment info
```

## Next Steps

- [Configuration Guide](configuration.md)
- [AI Providers Setup](ai-providers.md)
- [Plugin Development](plugins.md)
- [Runbook Guide](runbooks.md)
- [Safety Engine](safety.md)
