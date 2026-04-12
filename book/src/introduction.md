# DeftShell

**The AI-Powered Context-Aware Terminal for Developers.**

DeftShell (`ds`) makes your terminal intelligent — it auto-detects project context, integrates with multiple AI providers, intercepts dangerous commands, learns your patterns, and provides a plugin ecosystem.

## Features

- **Context Detection** — Auto-detects language, framework, package manager, services, and cloud provider from project files
- **AI Gateway** — Multi-provider support (Anthropic, OpenAI, Google Gemini, Ollama, GitHub Copilot, AWS Bedrock) with automatic fallback
- **Safety Engine** — Intercepts dangerous commands (`rm -rf /`, `git push --force`, `DROP TABLE`) with configurable risk levels
- **Command Intelligence** — Typo correction, alias suggestions, command sequence detection, and analytics
- **Runbook Manager** — Define, record, and replay multi-step workflows with variables and conditions
- **Plugin System** — Extend DeftShell with community plugins via npm
- **Smart Prompt** — Git status, stack info, execution time, and AI status in your prompt
- **Privacy Mode** — Route all AI queries through local models (Ollama) when enabled

## Quick Start

```bash
# Install
cargo install --git https://github.com/utsabpanta/deftshell.git ds-cli

# Add to your shell
eval "$(ds init zsh)"   # or bash/fish

# Try it out
ds ask "how do I rebase onto main?"
ds do "find all TODO comments in src/"
ds context              # See detected project info
ds chat                 # Interactive AI chat
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    DeftShell (ds)                      │
├─────────────────────────────────────────────────────┤
│  Shell Integration │ Context Engine │ AI Gateway     │
│  Command Intel     │ Plugin System  │ Safety Engine  │
│  Runbook Manager   │ Observations   │ Config Manager │
├─────────────────────────────────────────────────────┤
│              SQLite Storage + Keychain               │
└─────────────────────────────────────────────────────┘
```

Built as a Rust workspace:
- **`ds-core`** — Core library with all modules
- **`ds-cli`** — CLI binary with clap
- **`ds-plugin-sdk`** — Rust plugin SDK
