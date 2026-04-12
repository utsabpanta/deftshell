# Runbook Guide

Runbooks are reusable, multi-step workflows defined in TOML. They let you codify common procedures like deployments, database migrations, and environment setup.

## Creating a Runbook

```bash
ds runbook new deploy-staging
```

This creates `~/.deftshell/runbooks/deploy-staging.toml`:

```toml
[runbook]
name = "deploy-staging"
description = "Deploy to staging environment"
version = "1.0.0"
author = "Your Name"

[[steps]]
name = "Run tests"
command = "cargo test --workspace"
description = "Ensure all tests pass before deploying"

[[steps]]
name = "Build release"
command = "cargo build --release"

[[steps]]
name = "Deploy"
command = "rsync -avz target/release/app {{server}}:/opt/app/"
description = "Deploy binary to the staging server"

[variables]
server = { description = "Staging server hostname", default = "staging.example.com" }
```

## Running a Runbook

```bash
ds runbook run deploy-staging
ds runbook run deploy-staging --var server=staging2.example.com
ds runbook run deploy-staging --from-step 2  # Skip first step
```

## Recording a Runbook

Record commands as you type them:

```bash
ds runbook record my-setup    # Start recording
# ... run your commands normally ...
ds runbook stop               # Stop and save
```

## Variables

Define variables with defaults and descriptions:

```toml
[variables]
env = { description = "Target environment", default = "staging" }
tag = { description = "Docker image tag", required = true }
```

Use them in commands with `{{variable_name}}`:

```toml
[[steps]]
command = "docker push myapp:{{tag}}"
```

## Conditional Steps

```toml
[[steps]]
name = "Run migrations"
command = "cargo run -- migrate"
condition = "{{run_migrations}}"

[variables]
run_migrations = { description = "Run database migrations?", default = "true" }
```

## AI-Generated Runbooks

Let AI create a runbook from a description:

```bash
ds runbook generate "set up a new Python Django project with PostgreSQL and Docker"
```

## Runbook Commands

```bash
ds runbook new <name>           # Create a new runbook
ds runbook edit <name>          # Edit in $EDITOR
ds runbook delete <name>        # Delete a runbook
ds runbook list                 # List all runbooks
ds runbook show <name>          # Display steps
ds runbook run <name>           # Execute a runbook
ds runbook record [name]        # Start recording
ds runbook stop                 # Stop recording
ds runbook generate <desc>      # AI-generate a runbook
ds runbook search <query>       # Search community registry
ds runbook install <spec>       # Install from registry
ds runbook publish <name>       # Publish to registry
ds runbook trending             # Show trending runbooks
```
