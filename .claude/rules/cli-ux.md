---
paths:
  - "crates/ds-cli/**/*.rs"
---

# CLI UX Conventions

- Data output goes to stdout (JSON exports, config values, context export)
- Status, progress, and errors go to stderr (`eprintln!`)
- Destructive operations (delete, reset, remove) require confirmation via `dialoguer::Confirm`
- Empty state: always show a helpful message with a suggested next command
- Use consistent color coding: green=success, red=error, yellow=warning, cyan=values, dimmed=secondary
- Exit code 0 for success, 1 for errors, non-zero for safety warnings
- Long operations (AI requests, plugin installs) should print a status message before starting
- All commands must handle the `--yes` / auto_confirm flag to skip prompts in scripts
