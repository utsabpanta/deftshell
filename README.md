# DeftShell (`ds`)

**The AI-powered, context-aware shell layer for developers.**

[Documentation](https://utsabpanta.github.io/deftshell/) | [Guide](https://utsabpanta.github.io/deftshell/) | [Contributing](CONTRIBUTING.md)

DeftShell sits on top of your existing shell (Zsh, Bash, or Fish) and makes it intelligent — it auto-detects your project stack, connects to 6 AI providers, warns about dangerous commands before they run, learns your patterns, and lets you define reusable workflows.

> Think of it as what GitHub Copilot is to your editor, but for your terminal.

**AI is optional.** Stack detection, the safety engine, smart prompt, runbooks, plugins, analytics, and shell integration all work fully offline — no API key required. AI only powers a small group of commands (`ask`, `do`, `how`, `explain`, `review`, `chat`, `generate`, `runbook generate`). See the [Commands](#commands) section for the breakdown.

## Why DeftShell?

- **Zero config to start.** Run `eval "$(ds init zsh)"` and it works. Context detection, safety checks, and smart prompt happen automatically.
- **AI that knows your project.** When you run `ds ask`, the AI already knows your language, framework, services, and git state — no copy-pasting context.
- **6 AI providers, one interface.** Anthropic, OpenAI, Gemini, Ollama, GitHub Copilot, and AWS Bedrock. Switch with a flag (`--provider ollama`). Set up fallback chains.
- **Safety net built in.** 29 built-in rules warn about `rm -rf /`, `git push --force main`, `DROP TABLE`, and more — before they execute. Context-aware risk elevation on production branches.
- **Privacy mode.** One command (`ds privacy on`) routes all AI through local Ollama. Nothing leaves your machine.
- **Pure Rust.** Single static binary, fast startup, no runtime dependencies.

## Features

| Feature | What it does |
|---------|-------------|
| **Context Detection** | Auto-detects language, framework, package manager, services, CI/CD, and cloud provider from project files |
| **AI Gateway** | Multi-provider AI with automatic fallback (Anthropic, OpenAI, Gemini, Ollama, GitHub Copilot, AWS Bedrock) |
| **Safety Engine** | Warns about dangerous commands with configurable risk levels and context-aware elevation |
| **Command Intelligence** | Typo correction, alias suggestions, command sequence detection, and analytics |
| **Runbooks** | Define, record, and replay multi-step workflows (deploy scripts, setup procedures) with variables |
| **Plugin System** | Extend DeftShell with JavaScript plugins distributed via npm |
| **Smart Prompt** | Git branch/status, detected framework, environment, execution time — all in your prompt |
| **Privacy Mode** | Route all AI queries through local models (Ollama) when enabled |

## Installation

Pick one method, then follow the shell setup below.

### Option A: Quick Install Script (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/utsabpanta/deftshell/main/scripts/install.sh | sh
```

This downloads a prebuilt binary for your platform and installs it to `/usr/local/bin`. Requires a [tagged release](https://github.com/utsabpanta/deftshell/releases).

### Option B: Install via Cargo

```bash
cargo install --git https://github.com/utsabpanta/deftshell.git ds-cli
```

This builds from source and installs the `ds` binary to `~/.cargo/bin/`. Requires [Rust 1.75+](https://rustup.rs).

### Option C: Build from Source

```bash
git clone https://github.com/utsabpanta/deftshell.git
cd deftshell
cargo build --release
```

The binary is at `target/release/ds`. Add it to your PATH:

```bash
# Add to ~/.zshrc or ~/.bashrc
export PATH="/path/to/deftshell/target/release:$PATH"
```

### Shell Setup (required after any install method)

Add **one** of these lines to your shell config, then restart your shell:

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

This registers shell hooks for safety warnings, command tracking, context detection on `cd`, and the smart prompt.

### Verify Installation

```bash
ds version    # Should print v0.1.0
ds doctor     # Checks that everything is wired up
```

## Quick Start

### 1. Set Up an AI Provider

The fastest path if you already have GitHub CLI:

```bash
gh auth login           # If not already logged in
ds auth copilot         # Auto-detects token, sets as default
```

Or use any other provider:

```bash
ds auth anthropic       # Prompts for API key, stores securely
ds auth openai
ds auth gemini
ds auth bedrock         # AWS profile setup
```

For fully local/private AI:

```bash
ollama serve && ollama pull llama3.1
ds config set ai.default_provider ollama
```

### 2. Use It

```bash
# AI with project context
ds ask "how do I set up the database for this project?"
ds do "find all TODO comments in src/"
ds how "deploy this to production"

# Pipe output to AI
cat error.log | ds explain
git diff --staged | ds review

# Interactive chat (can execute code blocks from AI responses)
ds chat

# See what DeftShell detected about your project
ds context
ds scripts
ds env
```

## Commands

Commands marked **[AI]** require a configured AI provider (see [AI Provider Setup](#ai-provider-setup)). Everything else works offline with zero credentials.

### AI Commands (require a provider)

| Command | Description |
|---------|-------------|
| `ds ask <query>` | **[AI]** Ask a question with automatic project context |
| `ds do <instruction>` | **[AI]** AI generates a shell command, you confirm, it runs |
| `ds how <question>` | **[AI]** Get project-aware how-to instructions |
| `ds explain` | **[AI]** Pipe command output to AI for explanation |
| `ds review` | **[AI]** Pipe diffs to AI for code review |
| `ds chat` | **[AI]** Interactive AI chat with conversation history |
| `ds generate <type> [name]` | **[AI]** Generate boilerplate (component, migration, test, dockerfile, github-action) |

### Context Commands

| Command | Description |
|---------|-------------|
| `ds context` | Show detected project stack |
| `ds context refresh` | Force re-detection |
| `ds context export` | Export context as JSON |
| `ds scripts` | List runnable scripts (package.json, Makefile, etc.) |
| `ds run <script>` | Run a detected project script |
| `ds workspace list` | List monorepo packages |
| `ds env` | Show environment info (runtimes, git, AI status) |

### Runbook Commands

All runbook commands work offline *except* `ds runbook generate`.

| Command | Description |
|---------|-------------|
| `ds runbook new <name>` | Create a new runbook |
| `ds runbook run <name>` | Execute a runbook (with variables and step confirmation) |
| `ds runbook record` | Start recording your commands as a runbook |
| `ds runbook stop` | Stop recording and save |
| `ds runbook list` | List all runbooks |
| `ds runbook generate <desc>` | **[AI]** Generate a runbook from a description |

### Management Commands

| Command | Description |
|---------|-------------|
| `ds config` | Open config in `$EDITOR`, or use `get`/`set` subcommands |
| `ds auth <provider>` | Authenticate an AI provider |
| `ds auth status` | Show auth status for all providers |
| `ds stats [today\|week]` | Command analytics dashboard |
| `ds usage` | AI token usage by provider and period |
| `ds plugin list` | Manage plugins (install, remove, enable, disable) |
| `ds alias add name=cmd` | Manage shell aliases |
| `ds doctor` | Diagnose setup issues |
| `ds privacy <on\|off>` | Toggle privacy mode (local AI only) |

## AI Provider Setup

DeftShell supports 6 providers. Use `ds auth <provider>` for guided setup, or set env vars directly:

| Provider | Auth | Default Model |
|----------|------|---------------|
| **GitHub Copilot** | `gh auth login` (auto-detected) | gpt-4o |
| **Anthropic** | `ANTHROPIC_API_KEY` or `ds auth anthropic` | claude-sonnet-4-20250514 |
| **OpenAI** | `OPENAI_API_KEY` or `ds auth openai` | gpt-4o |
| **Gemini** | `GEMINI_API_KEY` or `ds auth gemini` | gemini-pro |
| **Ollama** | None (localhost) | llama3.1 |
| **AWS Bedrock** | AWS credentials + `ds auth bedrock` | anthropic.claude-3-sonnet |

Override for a single command: `ds ask "hello" --provider ollama`

Set up automatic fallback:

```toml
# ~/.deftshell/config.toml
[ai]
default_provider = "anthropic"
fallback_provider = "ollama"
```

## Safety Engine

DeftShell warns about dangerous commands before execution via shell hooks:

```
$ rm -rf /
  ⚠ CRITICAL: Destructive Command Detected
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Command:  rm -rf /
  Risk:     CRITICAL
  Reason:   Recursive force deletion of root directory
  Suggestion: Specify the exact directory you want to remove
```

29 built-in rules across 4 risk levels (Critical, High, Medium, Low). Risk is automatically elevated on protected branches (`main`, `production`) and in production Kubernetes contexts.

Add custom rules in `.deftshell.toml`:

```toml
[[safety.custom_rules.rule]]
pattern = "prisma migrate reset"
level = "high"
message = "This will reset the entire database."
```

## Configuration

Hierarchical TOML config — each layer overrides the previous:

1. Built-in defaults
2. `~/.deftshell/config.toml` (global)
3. `.deftshell.toml` (project-level, commit to repo)
4. Environment variables (`DS_*`)
5. CLI flags (`--provider`, `--yes`, etc.)

```toml
# ~/.deftshell/config.toml
[ai]
default_provider = "copilot"
fallback_provider = "ollama"

[prompt]
theme = "default"           # default, minimal, powerline, pure
show_git = true
show_execution_time = true

[safety]
enabled = true
confirm_threshold = "medium"
```

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                     DeftShell (ds)                    │
├──────────────────────────────────────────────────────┤
│  Shell Integration │ Context Engine  │ AI Gateway     │
│  Command Intel     │ Plugin System   │ Safety Engine  │
│  Runbook Manager   │ Smart Prompt    │ Config Manager │
├──────────────────────────────────────────────────────┤
│              SQLite Storage + Keychain                │
└──────────────────────────────────────────────────────┘
```

Built as a Rust workspace:

- **`ds-core`** — Core library (AI gateway, context detection, safety engine, runbooks, shell integration, storage)
- **`ds-cli`** — CLI binary (clap-based, 30+ subcommands)
- **`ds-plugin-sdk`** — Rust plugin SDK

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup. Quick start:

```bash
cargo build --workspace             # Build
cargo test --workspace              # Test (193 tests)
cargo clippy --workspace -- -D warnings  # Lint
```

## License

MIT — see [LICENSE](LICENSE) for details.
