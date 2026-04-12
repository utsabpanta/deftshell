# Context Commands

DeftShell auto-detects your project's language, framework, package manager, services, and infrastructure by scanning files in the current directory.

## Overview

| Command | Description |
|---------|-------------|
| `ds context` | Show detected project context |
| `ds context refresh` | Force re-detection |
| `ds context export` | Export context as JSON |
| `ds context diff` | Show changes since last detection |
| `ds scripts` | List detected scripts |
| `ds run <script>` | Run a project script |
| `ds workspace list` | List monorepo packages |
| `ds env` | Show environment info |

## `ds context` — View Detected Context

```bash
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

## `ds context export` — Export as JSON

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

## `ds scripts` — List Project Scripts

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

## `ds run` — Run a Script

```bash
ds run dev       # Runs the "dev" script
ds run test      # Runs the "test" script
```

## `ds workspace` — Monorepo Workspaces

```bash
ds workspace list    # List all packages in a monorepo
```

Supports: npm workspaces, pnpm workspaces, yarn workspaces, Cargo workspaces, Turborepo, Nx, Lerna.

## `ds env` — Environment Info

```bash
ds env    # Show environment context
```
