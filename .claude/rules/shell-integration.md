---
paths:
  - "crates/ds-core/src/shell/**/*.rs"
---

# Shell Integration Rules

- Each shell script (zsh.rs, bash.rs, fish.rs) generates a complete init script as a string
- All shells must include a double-load guard (`_DEFTSHELL_LOADED`) to prevent duplicate hooks
- The preexec hook must capture the command text BEFORE the safety check
- The precmd hook must capture `$?` (exit code) as its VERY FIRST operation
- Background commands (`ds track-command`, `ds context --detect`) use `&!` (zsh), `&` (bash), or `&` (fish) to avoid blocking
- Duration tracking: zsh uses `EPOCHREALTIME`, bash uses `SECONDS`, fish uses `date +%s`
- Never shell out to `git` — use `git2` crate in prompt.rs
- Alias loading: shell scripts must source `~/.deftshell/aliases.{shell}` if it exists
- Test shell scripts via integration tests that check for key strings (guard, hooks, safety-check call)
