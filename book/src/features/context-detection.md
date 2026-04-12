# Context Detection

DeftShell auto-detects your project's language, framework, package manager, services, and infrastructure by scanning files in the current directory.

## Detection Pipeline

The detection engine (`ContextDetector::detect()`) runs a **12-stage pipeline**:

1. **Explicit config** — Reads `.deftshell.toml` if present
2. **Package manifests** — Scans `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `Gemfile`, `pom.xml`, `build.gradle`, `composer.json`, `mix.exs`, `pubspec.yaml`, `Package.swift`
3. **Lock files** — Detects package manager from `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `Cargo.lock`, etc.
4. **Config files** — `tsconfig.json`, `Dockerfile`, `docker-compose.yml`, `Makefile`
5. **Directory structure** — Monorepo markers (`nx.json`, `turbo.json`, `lerna.json`)
6. **Environment files** — `.env`, `.env.local` — detects database URLs, service connections
7. **VCS** — Git repo, branch, remote (via `git2` crate, no subprocess)
8. **Runtime versions** — `.nvmrc`, `.node-version`, `.python-version`, `.ruby-version`, `.tool-versions`
9. **CI/CD** — `.github/workflows/`, `.gitlab-ci.yml`, `Jenkinsfile`, `azure-pipelines.yml`
10. **Cloud provider** — `serverless.yml`, `vercel.json`, Terraform files
11. **Docker** — Dockerfile, docker-compose services
12. **Services** — Infers database, cache, message queue from compose and env

## Framework Detection

For JavaScript/TypeScript projects, DeftShell detects **frameworks** by checking `dependencies` and `devDependencies` in `package.json`:

- React, Vue, Angular, Next.js, Nuxt, Astro, Gatsby, Svelte, NestJS, Fastify, Express, Remix

## Caching

Results are cached in SQLite (`context_cache` table) and invalidated when any watched file changes (checked via modification timestamps).

## Workspace Detection

DeftShell detects and supports monorepo workspace layouts:

- npm workspaces
- pnpm workspaces
- yarn workspaces
- Cargo workspaces
- Turborepo
- Nx
- Lerna
