# DeftShell Complete Guide

Everything DeftShell can do, with examples and how it works under the hood.

---

## Table of Contents

1. [Shell Integration](#1-shell-integration)
2. [Context Detection](#2-context-detection)
3. [AI Commands](#3-ai-commands)
4. [Safety Engine](#4-safety-engine)
5. [Command Intelligence](#5-command-intelligence)
6. [Runbooks](#6-runbooks)
7. [Plugin System](#7-plugin-system)
8. [Configuration & Auth](#8-configuration--auth)
9. [Analytics & Stats](#9-analytics--stats)
10. [Architecture & How It Works](#10-architecture--how-it-works)

---

## 1. Shell Integration

DeftShell hooks into your shell (Zsh, Bash, or Fish) to intercept commands, track history, and render a smart prompt.

### Commands

```bash
# Generate and load shell hooks
eval "$(ds init zsh)"      # Zsh
eval "$(ds init bash)"     # Bash
ds init fish | source      # Fish

# Generate tab completions
ds completions zsh > ~/.deftshell/_ds
ds completions bash > ~/.deftshell/ds.bash
ds completions fish > ~/.config/fish/completions/ds.fish
```

### How It Works

When you run `ds init zsh`, DeftShell outputs a shell script that registers 3 hooks:

**`preexec` (before every command):**
- Saves the command text to `_DEFTSHELL_LAST_CMD` for later tracking
- Starts a timer for duration tracking
- Calls `ds safety-check` to intercept dangerous commands before execution

**`precmd` (before every prompt):**
- Captures the exit code of the last command (`$?`) as the very first operation
- Calculates how long the last command took
- Calls `ds track-command` in the background to record the saved command
- Calls `ds prompt-segment` to render the smart prompt

**`chpwd` (on directory change):**
- Calls `ds context --detect --quiet` in the background to re-detect the project stack

**Shell-specific details:**
- **Zsh**: Uses `add-zsh-hook` for `precmd`/`preexec`, captures command via `$1` in preexec
- **Bash**: Uses `DEBUG` trap for preexec with a `_DEFTSHELL_PREEXEC_READY` guard to prevent re-firing on internal commands. `PROMPT_COMMAND` for precmd
- **Fish**: Uses `fish_preexec`/`fish_postexec` events. Exit code is captured as the first statement in postexec (`set -g _deftshell_last_exit $status`) to avoid clobbering by subsequent commands

All shells include a double-load guard (`_DEFTSHELL_LOADED`) to prevent duplicate hook registration.

**Code:** `crates/ds-core/src/shell/zsh.rs`, `bash.rs`, `fish.rs`

### Smart Prompt

The prompt shows contextual info:

```
✓ ~/projects/my-app (main*) [next] dev 1.2s >
│  │                 │       │      │   │
│  │                 │       │      │   └─ Last command duration (if >2s)
│  │                 │       │      └─ Environment (dev/staging/prod)
│  │                 │       └─ Detected framework
│  │                 └─ Git branch (* = dirty)
│  └─ Current directory
└─ Exit code (✓ = success, ✗ = failure)
```

4 themes available: `default`, `minimal`, `powerline`, `pure`

**Code:** `crates/ds-core/src/shell/prompt.rs` — Uses `git2` crate to read branch, dirty state, ahead/behind counts, and stash without shelling out to `git`.

---

## 2. Context Detection

DeftShell auto-detects your project's language, framework, package manager, services, and infrastructure by scanning files in the current directory.

### Commands

```bash
ds context              # Show what was detected
ds context refresh      # Force re-detection
ds context export       # Export as JSON
ds scripts              # List detected scripts (from package.json, Makefile, etc.)
ds run <script>         # Run a detected script
ds env                  # Show environment info
ds workspace list       # List monorepo packages
```

### Examples

```bash
$ cd ~/projects/my-nextjs-app
$ ds context
Project Context
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Project:        my-nextjs-app
  Root:           /Users/you/projects/my-nextjs-app

  Stack:
    Language:     typescript
    Framework:    next (15.2.0)
    Runtime:      node (22.14.0)
    Pkg Manager:  pnpm
    Test Runner:  vitest
    Linter:       eslint
    Bundler:      turbopack

  Infrastructure:
    Docker:       yes
    CI/CD:        github-actions
    Cloud:        vercel

  Services:
    Database:     postgresql
    Cache:        redis
```

```bash
$ ds context export
{
  "project": { "name": "my-nextjs-app", "root": "/Users/you/projects/my-nextjs-app" },
  "stack": {
    "primary_language": "typescript",
    "framework": "next",
    "framework_version": "14.1.0",
    "runtime": "node",
    "runtime_version": "20.11.0",
    "package_manager": "pnpm",
    "test_runner": "vitest",
    "linter": "eslint",
    "bundler": "turbopack"
  },
  ...
}
```

```bash
$ ds scripts
Project Scripts
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  build        next build
  dev          next dev --turbo
  lint         next lint
  start        next start
  test         vitest
```

### How It Works

The detection engine (`ContextDetector::detect()`) runs a **12-stage pipeline**:

1. **Explicit config** — Reads `.deftshell.toml` if present
2. **Package manifests** — Scans `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `Gemfile`, `pom.xml`, `build.gradle`, `composer.json`, `mix.exs`, `pubspec.yaml`, `Package.swift`
3. **Lock files** — Detects package manager from `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `Cargo.lock`, etc.
4. **Config files** — `tsconfig.json`, `Dockerfile`, `docker-compose.yml`, `Makefile`
5. **Directory structure** — Monorepo markers (`nx.json`, `turbo.json`, `lerna.json`)
6. **Environment files** — `.env`, `.env.local` → detects database URLs, service connections
7. **VCS** — Git repo, branch, remote (via `git2` crate, no subprocess)
8. **Runtime versions** — `.nvmrc`, `.node-version`, `.python-version`, `.ruby-version`, `.tool-versions`
9. **CI/CD** — `.github/workflows/`, `.gitlab-ci.yml`, `Jenkinsfile`, `azure-pipelines.yml`
10. **Cloud provider** — `serverless.yml`, `vercel.json`, Terraform files
11. **Docker** — Dockerfile, docker-compose services
12. **Services** — Infers database, cache, message queue from compose and env

For JavaScript/TypeScript projects, it detects **frameworks** by checking `dependencies` and `devDependencies` in `package.json`:
- React, Vue, Angular, Next.js, Nuxt, Astro, Gatsby, Svelte, NestJS, Fastify, Express, Remix

**Caching:** Results are cached in SQLite (`context_cache` table) and invalidated when any watched file changes (checked via modification timestamps).

**Workspace detection** supports: npm workspaces, pnpm workspaces, yarn workspaces, Cargo workspaces, Turborepo, Nx, Lerna.

**Code:** `crates/ds-core/src/context/detector.rs` (~600 lines), `workspace.rs`, `cache.rs`

---

## 3. AI Commands

DeftShell integrates with 6 AI providers to answer questions, generate commands, explain output, review code, and chat — all with automatic project context.

### Commands

```bash
# Ask a question with project context
ds ask "how do I set up the database for this project?"
ds ask "what does the UserService class do?"

# AI generates and runs commands
ds do "find all TODO comments in the codebase"
ds do "compress all PNG files in assets/ to under 500KB"
ds do "create a migration to add an email column to users"

# Get how-to instructions
ds how "deploy this to production"
ds how "set up Docker for local development"

# Explain piped output
cat error.log | ds explain
git diff HEAD~3 | ds explain

# Review code changes
git diff --staged | ds review

# Interactive chat (maintains conversation history)
ds chat
ds chat --continue           # Resume last conversation
ds chat --context src/app.rs # Include file in context
# Chat can execute shell commands from AI responses:
#   AI outputs a ```bash block → you're prompted to run it

# Code generation
ds generate component UserProfile
ds generate migration add-email-to-users
ds generate test UserService
ds generate dockerfile
ds generate github-action ci

# Override provider for a single command
ds ask "hello" --provider openai
ds ask "hello" --provider ollama
```

### How It Works

**AI Gateway** (`crates/ds-core/src/ai/gateway.rs`):
- Maintains a registry of providers, each implementing the `AiProvider` trait
- On each request: tries the default provider, falls back to fallback_provider on error
- **Privacy mode**: when enabled, routes all requests through a local provider (Ollama)

**Provider trait:**
```rust
#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    async fn complete(&self, request: &AiRequest) -> Result<AiResponse>;
    async fn stream(&self, request: &AiRequest) -> Result<Pin<Box<dyn Stream<...>>>>;
}
```

**6 providers implemented:**

| Provider | API | Auth | Default Model |
|----------|-----|------|---------------|
| Anthropic | Messages API v1 | `ANTHROPIC_API_KEY` | claude-sonnet-4-20250514 |
| OpenAI | Chat Completions | `OPENAI_API_KEY` | gpt-4o |
| Ollama | Local REST API | None (localhost) | llama3.1 |
| Gemini | GenerateContent | `GEMINI_API_KEY` | gemini-pro |
| Copilot | GitHub Models API | GitHub CLI (`gh auth login`) or VS Code Copilot extension | gpt-4o |
| Bedrock | AWS InvokeModel | AWS credentials + SigV4 signing | anthropic.claude-3-sonnet |

**Context injection:** The `ds ask` command builds a system prompt that includes the detected `StackProfile` (language, framework, services, etc.) so the AI knows about your project.

**Streaming:** All providers support SSE (Server-Sent Events) streaming for real-time output in `ds chat`.

**Chat command execution:** When the AI responds with fenced code blocks tagged as shell commands (`bash`, `sh`, `zsh`, `shell`, or untagged), `ds chat` offers to execute them directly:
- Each detected command is shown with a `[y]es / [n]o / [a]ll` prompt
- Commands are safety-checked through the safety engine before execution
- Dangerous commands show a warning but still allow execution if confirmed
- Execution results are displayed inline, and commands are recorded to history
- Leading `$ ` prompt markers in code blocks are automatically stripped

**Token tracking:** Every AI call records estimated `tokens_in` and `tokens_out` to the `ai_usage` SQLite table. For streaming responses (most commands), tokens are estimated from content length (~4 chars per token). Non-streaming calls (`ds do`) use the provider's reported token counts. Cost tracking is not yet implemented.

**Code:** `crates/ds-core/src/ai/gateway.rs`, `providers/anthropic.rs`, `openai.rs`, `ollama.rs`, `gemini.rs`, `copilot.rs`, `bedrock.rs`

---

## 4. Safety Engine

DeftShell warns about dangerous commands before they execute, showing risk levels and safer alternatives. The safety engine is advisory — it displays warnings via the shell `preexec` hook but does not block command execution.

### Commands

```bash
# Manually check a command
ds safety-check "rm -rf /"
ds safety-check "git push --force origin main"
ds safety-check "DROP TABLE users"
ds safety-check "docker system prune -a"
ds safety-check "kubectl delete namespace production"

# Safe commands pass through silently
ds safety-check "git status"        # No output, exit 0
ds safety-check "ls -la"            # No output, exit 0
```

### Example Output

```
$ ds safety-check "rm -rf /"

  ⚠ CAUTION: Destructive Command Detected
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Command:  rm -rf /
  Risk:     CRITICAL
  Reason:   Recursive forced deletion of the root filesystem

  Suggestion: Specify the exact directory you want to remove instead of /
```

```
$ ds safety-check "git push --force origin main"

  ⚠ CAUTION: Destructive Command Detected
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Command:  git push --force origin main
  Risk:     HIGH
  Reason:   Force pushing can overwrite remote history and cause data loss for collaborators

  Suggestion: Use --force-with-lease instead for safer force pushes
```

### How It Works

**Three-layer architecture:**

**Layer 1 — Rules** (`safety/rules.rs`): 29 built-in regex rules across 4 risk levels:

| Level | Count | Examples |
|-------|-------|---------|
| Critical | 11 | `rm -rf /`, `rm -rf ~`, `chmod 777 /`, `dd if=/dev/zero`, `mkfs`, fork bombs, `curl\|sh` |
| High | 12 | `git push --force`, `git reset --hard`, `DROP TABLE`, `docker system prune -a`, `terraform destroy`, `kubectl delete namespace` |
| Medium | 6 | `rm -rf node_modules`, `git checkout -- .`, `chmod -R`, `chown -R` |
| Low | 0 | (reserved for custom rules) |

Each rule is a compiled regex pattern. Example for detecting `rm -rf /`:
```regex
rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|
      (-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+)/\s*$
```
This matches `rm -rf /`, `rm -fr /`, `rm -r -f /`, etc.

**Layer 2 — Interceptor** (`safety/interceptor.rs`): Orchestrates the check:
1. If safety disabled → pass
2. Check allowlist (configurable patterns that always pass)
3. Check denylist (configurable patterns that always block as CRITICAL)
4. Match against all rules, return the highest-severity match

**Layer 3 — Risk Assessor** (`safety/assessor.rs`): Context-aware elevation that can only **increase** risk, never decrease:

| Context | Condition | Effect |
|---------|-----------|--------|
| Protected branch | On `main`/`master`/`production`/`prod`/`release` | Git + DB ops elevated 1 level |
| Production env | `is_production_env` flag | DB + infra ops elevated 1 level |
| K8s production | Context contains "prod"/"production"/"prd"/"live" | Everything → CRITICAL |
| Uncommitted changes | `has_uncommitted_changes` | Destructive git ops elevated 1 level |

**Shell integration:** The `preexec` hook calls `ds safety-check "$cmd"` before every command. If it returns non-zero, the warning is displayed.

**Code:** `crates/ds-core/src/safety/rules.rs`, `interceptor.rs`, `assessor.rs` — 42 tests cover this module

---

## 5. Command Intelligence

DeftShell learns from your command history to detect typos, suggest aliases, and identify command sequences.

### Commands

```bash
# These happen automatically via shell hooks:
# - Typo detection: "gti status" → "Did you mean `git status`?"
# - Alias suggestions: after running "docker-compose up -d --build" 20+ times
# - Sequence detection: "cargo build" always followed by "cargo test"

# View analytics
ds stats                    # Project stats
ds stats today              # Today's usage
ds stats week               # This week
ds stats --format json      # Export as JSON

# View AI usage
ds usage
```

### How It Works

**Typo Detection** (`intelligence/suggestions.rs`):

Uses two algorithms together:

1. **Fuzzy matching** (via `fuzzy-matcher` crate's Skim algorithm) — scores how similar the typed command is to known commands
2. **Levenshtein edit distance** — counts minimum single-character edits (insert, delete, replace) to transform one string into another

```
"gti" vs "git" → edit distance = 2 (two replacements), fuzzy score > 10 → suggests "git"
"dockre" vs "docker" → edit distance = 2, fuzzy score > 10 → suggests "docker"
"cargo" vs "cargo" → edit distance = 0 → exact match, not a typo
```

A command is flagged as a typo when: `edit_distance ≤ 2 AND edit_distance > 0 AND fuzzy_score > 10`

Checked against 30 common commands: `git`, `docker`, `npm`, `yarn`, `pnpm`, `cargo`, `python`, `pip`, `node`, `npx`, `kubectl`, `terraform`, `make`, `curl`, `wget`, `ssh`, `scp`, `rsync`, `find`, `grep`, `awk`, `sed`, `cat`, `ls`, `cd`, `mkdir`, `rm`, `cp`, `mv`

**Alias Suggestions**: If you run the same command 20+ times and it's longer than 15 characters, DeftShell suggests creating an alias. The alias name is auto-generated from the first letter of each word (up to 4): `"npm run test"` → `nrt`

**Sequence Detection**: Analyzes your last 100 commands per directory. If command B follows command A at least 5 times, it suggests running B automatically after A.

**Command Tracking** (`intelligence/tracker.rs`):
- Every command is recorded to SQLite via the shell `precmd` hook
- **Sensitive data redacted** before storage using regex patterns:
  - `PASSWORD=secret123` → `PASSWORD=***`
  - `Bearer abc123` → `Bearer ***`
  - `-p mypassword` → `-p ***`

**Code:** `crates/ds-core/src/intelligence/suggestions.rs`, `tracker.rs`, `analytics.rs`

---

## 6. Runbooks

Runbooks are reusable, multi-step workflows defined in TOML. Record, create, share, and replay procedures like deployments, database migrations, and environment setup.

### Commands

```bash
# Create and manage
ds runbook new deploy-staging       # Create new runbook
ds runbook edit deploy-staging      # Edit in $EDITOR
ds runbook show deploy-staging      # Display steps
ds runbook list                     # List all runbooks
ds runbook delete deploy-staging    # Delete

# Execute
ds runbook run deploy-staging                          # Run all steps
ds runbook run deploy-staging --from-step 2            # Skip first steps
ds runbook run deploy-staging --var server=staging2    # Pass variables

# Record commands as you work
ds runbook record my-setup          # Start recording
# ... run your commands normally ...
ds runbook stop                     # Stop and save

# AI-generate from description
ds runbook generate "set up a Python Django project with PostgreSQL and Docker"
```

### Runbook Format

```toml
[runbook]
name = "deploy-staging"
title = "Deploy to Staging"
description = "Build, test, and deploy to staging"
author = "your-name"
version = "1.0.0"
tags = ["deploy", "staging"]
estimated_time = "5m"
requires = ["docker", "kubectl"]

[[steps]]
title = "Run tests"
command = "cargo test --workspace"
confirm = true
on_failure = "abort"

[[steps]]
title = "Build image"
command = "docker build -t myapp:{{tag}} ."
on_failure = "abort"

[[steps]]
title = "Push image"
command = "docker push myapp:{{tag}}"
on_failure = "retry"

[[steps]]
title = "Deploy"
command = "kubectl set image deployment/myapp myapp=myapp:{{tag}} -n {{namespace}}"
on_failure = "abort"
fallback_command = "kubectl rollout undo deployment/myapp -n {{namespace}}"

[variables]
tag = { description = "Docker image tag", default = "latest" }
namespace = { description = "Kubernetes namespace", default = "staging" }
```

### How It Works

**Parser** (`runbook/parser.rs`): Deserializes TOML into `Runbook` struct with `RunbookStep` entries. Variables use `{{variable_name}}` syntax and are substituted at execution time.

**Executor** (`runbook/executor.rs`):
1. Iterate steps (optionally starting from `from_step`)
2. Substitute `{{variables}}` in command strings
3. If `dry_run` → show command without executing
4. If `confirm: true` → prompt user for confirmation
5. Execute via `sh -c "<command>"` (or `cmd /C` on Windows)
6. On failure, handle based on `on_failure`:
   - `abort` → Stop execution entirely
   - `skip` → Log as skipped, continue
   - `retry` → Retry once, then fail
7. If a `fallback_command` exists, try it on failure

**Recording:** When `ds runbook record` is active, every command you run in the shell is captured and saved as a step.

**Code:** `crates/ds-core/src/runbook/parser.rs`, `executor.rs`, `registry.rs`

---

## 7. Plugin System

Extend DeftShell with community plugins distributed via npm.

### Commands

```bash
ds plugin list                      # List installed plugins
ds plugin install deftshell-k8s      # Install from npm
ds plugin install ./my-local-plugin # Install from local path
ds plugin remove deftshell-k8s       # Remove
ds plugin update                    # Update all plugins
ds plugin update deftshell-k8s       # Update specific
ds plugin search "kubernetes"       # Search npm registry
ds plugin enable deftshell-k8s       # Enable
ds plugin disable deftshell-k8s      # Disable (without removing)
ds plugin info deftshell-k8s         # Show details
ds plugin create my-plugin          # Scaffold new plugin
```

### Creating a Plugin

#### 1. Scaffold a new plugin

```bash
ds plugin create my-plugin    # generates a starter project in ~/.deftshell/plugins/
```

Or start from scratch — create a directory with a `package.json`:

```json
{
  "name": "deftshell-my-plugin",
  "version": "0.1.0",
  "main": "index.js",
  "keywords": ["deftshell-plugin"],
  "deftshell": {
    "type": "command"
  }
}
```

Valid plugin types: `stack-pack`, `ai-provider`, `theme`, `command`, `safety-rule`, `integration`.

#### 2. Write the plugin

```javascript
// index.js
module.exports = {
  name: 'deftshell-my-plugin',
  version: '0.1.0',
  type: 'command',
  description: 'My custom DeftShell plugin',
  author: 'Your Name',

  async onActivate(context) {
    console.log('Plugin activated');
  },

  async onDeactivate() {
    // Cleanup resources
  },

  commands: [
    {
      name: 'deploy',
      description: 'Deploy the current project',
      async handler(args, context) {
        const env = args[0] || 'staging';
        console.log(`Deploying to ${env}...`);
      },
    },
  ],

  safetyRules: [
    {
      name: 'no-force-push-main',
      pattern: 'git push --force.*main',
      level: 'critical',
      message: 'Force-pushing to main is blocked by plugin policy',
    },
  ],
};
```

#### 3. Install and run

```bash
ds plugin install ./my-plugin     # install from local path
ds deploy production              # run your plugin command
```

To publish on npm so others can install with `ds plugin install deftshell-my-plugin`,
include `"deftshell-plugin"` in your `keywords` array.

### How It Works

**Plugin Loader** (`plugin/loader.rs`): Scans `~/.deftshell/plugins/` for directories containing `package.json` or `plugin.toml`. A `.disabled` marker file controls enable/disable without removing (`ds plugin disable <name>`).

**Plugin Runtime** (`plugin/runtime.rs`): Plugins run as Node.js subprocesses. DeftShell invokes `node <entry_point> <command> <args>` and captures stdout.

**Plugin Manifest** supports two formats:
- `package.json` with `"keywords": ["deftshell-plugin"]` and a `"deftshell": { "type": "..." }` field
- `plugin.toml` with plugin metadata fields

**Code:** `crates/ds-core/src/plugin/loader.rs`, `runtime.rs`, `registry.rs`

---

## 8. Configuration & Auth

### Quick Start: Setting Up AI

DeftShell supports 6 AI providers. Here's how to get started with each:

**GitHub Copilot** (easiest if you already have GitHub CLI at work):
```bash
# If you have GitHub CLI installed and logged in:
ds auth copilot                            # Auto-detects gh token, sets as default
ds ask "hello"                             # Just works!

# That's it. If you're not logged in yet:
gh auth login                              # One-time: log in to GitHub
ds auth copilot                            # Sets Copilot as default

# Uses the GitHub Models API (GPT-4o, etc.) with your GitHub token.
# No extra scopes, no manual config needed.
```

**Anthropic (Claude)**:
```bash
ds auth anthropic                          # Prompts for API key, stores in credential file
ds config set ai.default_provider anthropic
# Or set the env var: export ANTHROPIC_API_KEY=sk-ant-...
```

**OpenAI (GPT-4o)**:
```bash
ds auth openai                             # Prompts for API key
ds config set ai.default_provider openai
# Or: export OPENAI_API_KEY=sk-...
```

**Google Gemini**:
```bash
ds auth gemini                             # Prompts for API key
ds config set ai.default_provider gemini
# Or: export GEMINI_API_KEY=...
```

**Ollama** (local, private, free):
```bash
# Install: https://ollama.com
ollama serve                               # Start the server
ollama pull llama3.1                       # Download a model
ds config set ai.default_provider ollama
ds ask "hello"                             # Runs locally, no data leaves your machine
```

**AWS Bedrock** (Claude via AWS):
```bash
ds auth bedrock                            # Prompts for AWS profile name
ds config set ai.default_provider bedrock
# Requires: aws configure --profile <name>
```

**Per-command override** — use any provider without changing defaults:
```bash
ds ask "hello" --provider copilot
ds ask "hello" --provider ollama
```

### Configuration Commands

```bash
ds config                    # Open config in $EDITOR
ds config get ai.default_provider
ds config set ai.default_provider copilot
ds config set safety.confirm_threshold medium
ds config reset              # Reset to defaults
ds config validate           # Check for errors
ds config path               # Show file location (~/.deftshell/config.toml)
ds config export             # Export as JSON

# Privacy mode
ds privacy on                # Route all AI through local Ollama
ds privacy off

# Aliases
ds alias list
ds alias add deploy="kubectl apply -f k8s/"
ds alias remove deploy
```

### Auth Commands

```bash
ds auth status               # Show all provider auth status
ds auth anthropic            # Authenticate Anthropic (stores key in credential file)
ds auth openai               # Authenticate OpenAI
ds auth ollama               # Test Ollama connection + list models
ds auth copilot              # GitHub Copilot setup (auto-detects gh CLI)
ds auth bedrock              # AWS Bedrock profile setup
ds auth revoke anthropic     # Remove stored credentials
```

### How It Works

**Auto-registration:** All 6 providers are registered automatically. You only need to set `default_provider` and have credentials — no manual config file editing required. The gateway checks each provider's `is_available()` method to determine if it has valid credentials.

**Copilot auth flow** (`providers/copilot.rs`):
1. Gets your GitHub token from `~/.config/github-copilot/hosts.json`, `gh auth token`, or `GITHUB_TOKEN` env var
2. Uses it directly with the GitHub Models API at `https://models.inference.ai.azure.com`
3. No token exchange or special scopes required — any `gh auth login` token works

**Hierarchical config loading** (`config/loader.rs`):
1. Built-in defaults
2. `~/.deftshell/config.toml` (global)
3. `.deftshell.toml` (project, committed to repo)
4. Environment variables (`DS_*`)
5. CLI flags (`--provider`, `--yes`, etc.)

Each layer overrides the previous.

**Credential Store** (`storage/keychain.rs`): API keys are stored in a file-based credential store at `~/.deftshell/credentials.toml` with restrictive file permissions (0600 on Unix). This avoids OS keychain popups that occur with unsigned binaries on macOS. Format:

```toml
[auth]
anthropic_api_key = "sk-ant-..."
openai_api_key = "sk-..."
```

**Code:** `crates/ds-core/src/config/schema.rs` (full config structs), `loader.rs`, `crates/ds-core/src/storage/keychain.rs`

---

## 9. Analytics & Stats

### Commands

```bash
ds stats                    # Project command analytics
ds stats today              # Today's commands
ds stats week               # This week
ds stats --format json      # Export as JSON
ds stats --format csv       # Export as CSV
ds usage                    # AI token usage by provider and period
ds doctor                   # Diagnose setup issues
ds doctor --verbose         # Verbose diagnostics
ds version                  # Version info
```

### Example Output

```bash
$ ds stats
Analytics Dashboard (project)

  Commands
    Total:        847
    Unique:       63
    Error rate:   8%

  Most Used:
    1. git status          (142)
    2. cargo test          (98)
    3. cargo build         (87)
    4. git add -A          (54)
    5. git commit          (51)
```

```bash
$ ds doctor
DeftShell Doctor
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  [OK] ds binary in PATH
  [OK] Shell integration configured
  [OK] Git available
  [OK] Node.js available

AI Providers:
  [OK]   Ollama (local)
  [OK]   Anthropic API key
  [!!]   OpenAI API key (not set)

Storage:
  [OK] Database accessible
  [OK] Data directory exists
  [OK] User config file

  8 passed, 1 warning, 0 failed
```

### How It Works

**SQLite database** (`storage/db.rs`) with 5 tables:

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `command_history` | Every command you run | command, directory, exit_code, duration_ms, timestamp |
| `context_cache` | Cached project detection results | directory, context_json, cached_at |
| `ai_usage` | Token usage per AI call | provider, tokens_in, tokens_out, cost_cents |
| `settings` | Persisted user settings | key, value |

**Performance:** Uses WAL mode, busy timeout of 5s, and foreign keys enabled. All writes happen in the background (shell hooks use `&!` or `&`).

**TUI Dashboard** (`crates/ds-cli/src/tui/dashboard.rs`): Built with `ratatui` for the terminal UI with sparklines, bar charts, and color-coded tables.

**Code:** `crates/ds-core/src/storage/db.rs`, `migrations.rs`, `crates/ds-core/src/intelligence/analytics.rs`

---

## 10. Architecture & How It Works

### Crate Structure

```
deftshell/
├── crates/
│   ├── ds-cli/           # Binary crate — CLI entry point
│   │   ├── commands/     # One file per command (ask.rs, chat.rs, etc.)
│   │   └── tui/          # Terminal UI (ratatui dashboard + widgets)
│   ├── ds-core/          # Library crate — all business logic
│   │   ├── ai/           # AI gateway + 6 provider implementations
│   │   ├── config/       # TOML schema + hierarchical loader
│   │   ├── context/      # Project detection + workspace + cache
│   │   ├── intelligence/ # Command tracking, typo detection, analytics
│   │   ├── plugin/       # Plugin loader, runtime, manager
│   │   ├── runbook/      # Parser, executor, registry
│   │   ├── safety/       # Rules, interceptor, risk assessor
│   │   ├── shell/        # Init scripts + prompt renderer
│   │   └── storage/      # SQLite + credential store
│   └── ds-plugin-sdk/    # Rust plugin SDK (re-exports core types)
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` 4 | CLI argument parsing with derive macros |
| `tokio` | Async runtime for AI HTTP calls |
| `reqwest` | HTTP client (rustls-tls, streaming, blocking) |
| `rusqlite` | SQLite with bundled/vendored libsqlite3 |
| `git2` | Native Git operations (no subprocess) |
| `serde` / `toml` / `serde_json` | Serialization |
| `ratatui` + `crossterm` | Terminal UI |
| `colored` | Colored terminal output |
| `fuzzy-matcher` | Skim fuzzy matching for typo detection |
| `regex` | Safety rule pattern matching |
| `ring` | SHA-256 + HMAC for AWS SigV4 signing |
| `dialoguer` | Interactive prompts (confirm, password input) |
| `async-trait` | Async methods in traits (AI provider interface) |

### Design Patterns

**Trait-based providers:** All AI providers implement `AiProvider` trait — adding a new provider means implementing 4 methods. The gateway does provider selection, fallback, and privacy routing.

**Fail-safe risk elevation:** The safety assessor can only **increase** risk levels, never decrease them. This prevents context from accidentally making dangerous commands appear safe.

**Background operations:** Shell hooks run `ds track-command` and `ds context --detect` with `&!` (background, no job control) to avoid slowing down the terminal.

**Graceful degradation:** If the credential store file is missing or unreadable, operations silently return empty results. If a regex pattern is invalid, it's skipped with a warning. If the AI provider fails, the fallback is tried.

### Data Flow

```
User types command
       │
       ▼
   preexec hook ──→ ds safety-check ──→ Block or allow
       │
       ▼
   Command executes
       │
       ▼
   precmd hook ──┬──→ ds track-command (background) ──→ SQLite
                 │
                 ├──→ ds prompt-segment ──→ Render prompt
                 │
                 └──→ Observation check ──→ Suggest if triggered
```

### Test Coverage

193 tests across the workspace:

**Unit tests (94):**
- Safety engine: 30 tests (rules, interceptor, assessor)
- Context detection: 22 tests (languages, frameworks, workspaces)
- Storage: 8 tests (SQLite CRUD, migrations, credential store)
- Command intelligence: 3 tests (edit distance, alias naming)
- CLI commands: 8 tests (command parsing, TUI widgets)
- AI providers: 9 tests (Bedrock SigV4, utilities)
- Config: 3 tests (defaults, project parsing, env vars)
- Runbooks: 2 tests (parsing, variable substitution)
- Other: 9 tests (cache, context)

**CLI integration tests (43)** — `crates/ds-cli/tests/cli_integration.rs`:
- Invoke the compiled `ds` binary via `assert_cmd`
- Cover: version, help, init (zsh/bash/fish), completions, context detection, safety checks, config operations, auth status, input validation, stats, usage, doctor, aliases, env, scripts, runbooks, plugins, privacy, prompt segments, track-command, provider flag

**Core integration tests (56)** — `crates/ds-core/tests/integration.rs`:
- Context detection against 6 fixture project types (Rust, Next.js, Python/Django, Ruby/Rails, Go, monorepo)
- AI context builder (token budget, secret exclusion, error inclusion)
- Safety interceptor (safe/dangerous/allowlist/denylist/custom rules/disabled/production context)
- Database CRUD (commands, stats, context cache, AI usage)
- Config loading and serialization
- Runbook parsing, variable substitution, save/load
- Shell script generation (guards, safety hooks, alias loading, exit code handling)
- E2E: secrets never leak to AI context
