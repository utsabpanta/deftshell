# Management Commands

Commands for configuration, authentication, diagnostics, and system management.

## Configuration

```bash
ds config                    # Open config in $EDITOR
ds config get <key>          # Get a value
ds config set <key> <value>  # Set a value
ds config reset              # Reset to defaults
ds config validate           # Check for errors
ds config path               # Show file location
ds config export             # Export as JSON
```

## Authentication

```bash
ds auth status               # Show all provider auth status
ds auth anthropic            # Authenticate Anthropic
ds auth openai               # Authenticate OpenAI
ds auth ollama               # Test Ollama connection + list models
ds auth copilot              # GitHub Copilot setup (auto-detects gh CLI)
ds auth gemini               # Authenticate Google Gemini
ds auth bedrock              # AWS Bedrock profile setup
ds auth revoke <provider>    # Remove stored credentials
```

## Analytics & Stats

```bash
ds stats                    # Project command analytics
ds stats today              # Today's commands
ds stats week               # This week
ds stats --format json      # Export as JSON
ds stats --format csv       # Export as CSV
ds usage                    # AI token usage and estimated costs
```

Example output:

```
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

## Diagnostics

```bash
ds doctor                   # Diagnose setup issues
ds doctor --verbose         # Verbose diagnostics
ds version                  # Version info
```

## Aliases

```bash
ds alias list                                    # List aliases
ds alias add deploy="kubectl apply -f k8s/"      # Add alias
ds alias remove deploy                           # Remove alias
```

## Privacy Mode

```bash
ds privacy on     # Route all AI through local Ollama
ds privacy off    # Use cloud providers
```

## Plugin Management

```bash
ds plugin list              # List installed plugins
ds plugin install <name>    # Install from npm
ds plugin remove <name>     # Remove
ds plugin update            # Update all
ds plugin search <query>    # Search npm registry
ds plugin enable <name>     # Enable a plugin
ds plugin disable <name>    # Disable without removing
ds plugin info <name>       # Show details
ds plugin create <name>     # Scaffold new plugin
```
