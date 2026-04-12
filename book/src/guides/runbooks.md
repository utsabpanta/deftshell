# Runbooks

Runbooks are reusable, multi-step workflows defined in TOML. They let you codify common procedures like deployments, database migrations, and environment setup.

## Runbook Format

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

Pass variables at runtime:

```bash
ds runbook run deploy-staging --var tag=v1.2.3 --var namespace=production
```

## Step Options

Each step supports:

| Field | Description |
|-------|-------------|
| `title` | Step display name |
| `command` | Shell command to execute |
| `description` | Optional explanation |
| `confirm` | Ask for confirmation before running (default: false) |
| `on_failure` | `abort`, `skip`, or `retry` |
| `fallback_command` | Alternative command to run on failure |
| `condition` | Only run if variable is truthy |

## Conditional Steps

```toml
[[steps]]
name = "Run migrations"
command = "cargo run -- migrate"
condition = "{{run_migrations}}"

[variables]
run_migrations = { description = "Run database migrations?", default = "true" }
```

## Execution Flow

1. Iterate steps (optionally starting from `--from-step`)
2. Substitute `{{variables}}` in command strings
3. If `dry_run` → show command without executing
4. If `confirm: true` → prompt user for confirmation
5. Execute via `sh -c "<command>"`
6. On failure, handle based on `on_failure`:
   - `abort` → Stop execution entirely
   - `skip` → Log as skipped, continue
   - `retry` → Retry once, then fail
7. If a `fallback_command` exists, try it on failure

## Recording

Record commands as you work to create a runbook automatically:

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

## Community Registry

Share and discover runbooks:

```bash
ds runbook search "kubernetes deploy"
ds runbook install user/k8s-deploy
ds runbook publish my-runbook
ds runbook trending
```
