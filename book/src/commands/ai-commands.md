# AI Commands

DeftShell integrates with 6 AI providers to answer questions, generate commands, explain output, review code, and chat — all with automatic project context.

## Overview

| Command | Description |
|---------|-------------|
| `ds ask <query>` | Ask AI a question with project context |
| `ds do <instruction>` | AI generates and executes commands |
| `ds how <question>` | Get project-aware how-to instructions |
| `ds explain` | Explain piped command output |
| `ds review` | Review piped code changes |
| `ds chat` | Interactive AI chat mode |
| `ds generate <type> [name]` | AI code generation |

## `ds ask` — Ask Questions

Ask AI a question with full project context injected automatically:

```bash
ds ask "how do I set up the database for this project?"
ds ask "what does the UserService class do?"
```

## `ds do` — Generate and Execute Commands

AI generates and runs shell commands based on your instruction:

```bash
ds do "find all TODO comments in the codebase"
ds do "compress all PNG files in assets/ to under 500KB"
ds do "create a migration to add an email column to users"
```

## `ds how` — How-To Instructions

Get project-aware step-by-step instructions:

```bash
ds how "deploy this to production"
ds how "set up Docker for local development"
```

## `ds explain` — Explain Output

Pipe command output for AI explanation:

```bash
cat error.log | ds explain
git diff HEAD~3 | ds explain
```

## `ds review` — Code Review

Pipe code changes for AI review:

```bash
git diff --staged | ds review
```

## `ds chat` — Interactive Chat

Maintain a conversation with the AI, with full project context:

```bash
ds chat                          # Start new conversation
ds chat --continue               # Resume last conversation
ds chat --context src/app.rs     # Include file in context
```

When the AI responds with fenced code blocks tagged as shell commands, `ds chat` offers to execute them:
- Each detected command is shown with a `[y]es / [n]o / [a]ll` prompt
- Commands are safety-checked through the safety engine before execution
- Execution results are displayed inline

## `ds generate` — Code Generation

Generate code scaffolding using AI:

```bash
ds generate component UserProfile
ds generate migration add-email-to-users
ds generate test UserService
ds generate dockerfile
ds generate github-action ci
```

## Provider Override

Use any provider for a single command without changing defaults:

```bash
ds ask "hello" --provider openai
ds ask "hello" --provider ollama
```

## How It Works

**AI Gateway** (`crates/ds-core/src/ai/gateway.rs`):
- Maintains a registry of providers, each implementing the `AiProvider` trait
- On each request: tries the default provider, falls back to `fallback_provider` on error
- **Privacy mode**: when enabled, routes all requests through a local provider (Ollama)

**Context injection:** The `ds ask` command builds a system prompt that includes the detected `StackProfile` (language, framework, services, etc.) so the AI knows about your project.

**Streaming:** All providers support SSE (Server-Sent Events) streaming for real-time output in `ds chat`.

**Token tracking:** Every AI call records estimated `tokens_in`, `tokens_out`, and cost to the `ai_usage` SQLite table.
