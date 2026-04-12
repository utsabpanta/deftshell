# Runbook Commands

Runbooks are reusable, multi-step workflows defined in TOML. Record, create, share, and replay procedures like deployments, database migrations, and environment setup.

## Overview

| Command | Description |
|---------|-------------|
| `ds runbook new <name>` | Create a new runbook |
| `ds runbook edit <name>` | Edit in `$EDITOR` |
| `ds runbook show <name>` | Display steps |
| `ds runbook list` | List all runbooks |
| `ds runbook delete <name>` | Delete a runbook |
| `ds runbook run <name>` | Execute a runbook |
| `ds runbook record [name]` | Start recording commands |
| `ds runbook stop` | Stop recording |
| `ds runbook generate <desc>` | AI-generate a runbook |
| `ds runbook search <query>` | Search community registry |
| `ds runbook install <spec>` | Install from registry |
| `ds runbook publish <name>` | Publish to registry |
| `ds runbook trending` | Show trending runbooks |

## Creating Runbooks

```bash
ds runbook new deploy-staging
```

This creates `~/.deftshell/runbooks/deploy-staging.toml`.

## Running Runbooks

```bash
ds runbook run deploy-staging                          # Run all steps
ds runbook run deploy-staging --from-step 2            # Skip first steps
ds runbook run deploy-staging --var server=staging2    # Pass variables
```

## Recording Runbooks

Record commands as you type them:

```bash
ds runbook record my-setup    # Start recording
# ... run your commands normally ...
ds runbook stop               # Stop and save
```

## AI-Generated Runbooks

Let AI create a runbook from a natural language description:

```bash
ds runbook generate "set up a Python Django project with PostgreSQL and Docker"
```

## Community Runbooks

```bash
ds runbook search "kubernetes deploy"
ds runbook install user/k8s-deploy
ds runbook publish my-runbook
ds runbook trending
```

For details on runbook format, variables, and authoring, see the [Runbooks Guide](../guides/runbooks.md).
