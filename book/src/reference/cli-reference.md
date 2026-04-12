# CLI Reference

Complete reference of all `ds` commands.

## AI Commands

| Command | Description |
|---------|-------------|
| `ds ask <query>` | Ask AI a question with project context |
| `ds ask <query> --provider <name>` | Use a specific AI provider |
| `ds do <instruction>` | AI generates and executes commands |
| `ds how <question>` | Get project-aware how-to instructions |
| `ds explain` | Explain piped command output |
| `ds review` | Review piped code changes |
| `ds chat` | Start interactive AI chat |
| `ds chat --continue` | Resume previous conversation |
| `ds chat --context <file>` | Include file in chat context |
| `ds generate <type> [name]` | AI code generation |

## Context Commands

| Command | Description |
|---------|-------------|
| `ds context` | Show detected project context |
| `ds context refresh` | Force re-detection |
| `ds context export` | Export context as JSON |
| `ds context diff` | Show changes since last detection |
| `ds scripts` | List detected project scripts |
| `ds run <script>` | Run a project script |
| `ds workspace list` | List monorepo packages |
| `ds env` | Show environment info |

## Runbook Commands

| Command | Description |
|---------|-------------|
| `ds runbook new <name>` | Create a new runbook |
| `ds runbook edit <name>` | Edit in `$EDITOR` |
| `ds runbook show <name>` | Display steps |
| `ds runbook list` | List all runbooks |
| `ds runbook delete <name>` | Delete a runbook |
| `ds runbook run <name>` | Execute a runbook |
| `ds runbook run <name> --from-step <n>` | Start from step N |
| `ds runbook run <name> --var <key>=<val>` | Pass variable |
| `ds runbook record [name]` | Start recording commands |
| `ds runbook stop` | Stop recording |
| `ds runbook generate <desc>` | AI-generate a runbook |
| `ds runbook search <query>` | Search community registry |
| `ds runbook install <spec>` | Install from registry |
| `ds runbook publish <name>` | Publish to registry |
| `ds runbook trending` | Show trending runbooks |

## Configuration Commands

| Command | Description |
|---------|-------------|
| `ds config` | Open config in `$EDITOR` |
| `ds config get <key>` | Get a configuration value |
| `ds config set <key> <value>` | Set a configuration value |
| `ds config reset` | Reset to defaults |
| `ds config validate` | Validate current config |
| `ds config path` | Show config file path |
| `ds config export` | Export config as JSON |

## Auth Commands

| Command | Description |
|---------|-------------|
| `ds auth status` | Show all provider auth status |
| `ds auth anthropic` | Authenticate Anthropic |
| `ds auth openai` | Authenticate OpenAI |
| `ds auth gemini` | Authenticate Google Gemini |
| `ds auth ollama` | Test Ollama connection |
| `ds auth copilot` | GitHub Copilot setup |
| `ds auth bedrock` | AWS Bedrock setup |
| `ds auth revoke <provider>` | Remove stored credentials |

## Plugin Commands

| Command | Description |
|---------|-------------|
| `ds plugin list` | List installed plugins |
| `ds plugin install <name>` | Install from npm |
| `ds plugin install <path>` | Install from local path |
| `ds plugin remove <name>` | Remove a plugin |
| `ds plugin update` | Update all plugins |
| `ds plugin update <name>` | Update specific plugin |
| `ds plugin search <query>` | Search npm registry |
| `ds plugin enable <name>` | Enable a plugin |
| `ds plugin disable <name>` | Disable without removing |
| `ds plugin info <name>` | Show plugin details |
| `ds plugin create <name>` | Scaffold new plugin |

## Other Commands

| Command | Description |
|---------|-------------|
| `ds stats` | Project command analytics |
| `ds stats today` | Today's commands |
| `ds stats week` | This week's commands |
| `ds stats --format <fmt>` | Export stats (json, csv) |
| `ds usage` | AI token usage and costs |
| `ds doctor` | Diagnose setup issues |
| `ds doctor --verbose` | Verbose diagnostics |
| `ds version` | Show version info |
| `ds alias list` | List aliases |
| `ds alias add <name>=<cmd>` | Add an alias |
| `ds alias remove <name>` | Remove an alias |
| `ds privacy on` | Enable privacy mode |
| `ds privacy off` | Disable privacy mode |

## Shell Integration

| Command | Description |
|---------|-------------|
| `ds init zsh` | Generate Zsh integration script |
| `ds init bash` | Generate Bash integration script |
| `ds init fish` | Generate Fish integration script |
| `ds completions zsh` | Generate Zsh completions |
| `ds completions bash` | Generate Bash completions |
| `ds completions fish` | Generate Fish completions |

## Internal Commands

These are called by shell hooks and generally not invoked directly:

| Command | Description |
|---------|-------------|
| `ds safety-check <cmd>` | Check a command against safety rules |
| `ds track-command` | Record a command to history |
| `ds prompt-segment` | Render the smart prompt |
