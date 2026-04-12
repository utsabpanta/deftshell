# Shell Setup

DeftShell hooks into your shell to intercept commands, track history, and render a smart prompt.

## Zsh

Add to `~/.zshrc`:

```bash
eval "$(ds init zsh)"
```

## Bash

Add to `~/.bashrc`:

```bash
eval "$(ds init bash)"
```

## Fish

Add to `~/.config/fish/config.fish`:

```fish
ds init fish | source
```

Restart your shell or run `source ~/.zshrc` (or equivalent) to activate.

## Tab Completions

Generate shell completions for command-line tab completion:

```bash
ds completions zsh > ~/.deftshell/_ds
ds completions bash > ~/.deftshell/ds.bash
ds completions fish > ~/.config/fish/completions/ds.fish
```

## How Shell Integration Works

When you run `ds init zsh`, DeftShell outputs a shell script that registers 3 hooks:

**`preexec` (before every command):**
- Saves the command text for later tracking
- Starts a timer for duration tracking
- Calls `ds safety-check` to intercept dangerous commands before execution

**`precmd` (before every prompt):**
- Captures the exit code of the last command
- Calculates how long the last command took
- Calls `ds track-command` in the background to record the command
- Calls `ds prompt-segment` to render the smart prompt

**`chpwd` (on directory change):**
- Calls `ds context --detect --quiet` in the background to re-detect the project stack

### Shell-Specific Details

- **Zsh**: Uses `add-zsh-hook` for `precmd`/`preexec`, captures command via `$1` in preexec
- **Bash**: Uses `DEBUG` trap for preexec with a guard to prevent re-firing on internal commands. `PROMPT_COMMAND` for precmd
- **Fish**: Uses `fish_preexec`/`fish_postexec` events. Exit code is captured as the first statement in postexec to avoid clobbering

All shells include a double-load guard (`_DEFTSHELL_LOADED`) to prevent duplicate hook registration.

## First Steps

After shell setup, try these commands:

```bash
ds ask "what does this project do?"
ds how "set up a development database"
ds do "find all files larger than 10MB"
ds context       # See what DeftShell detected
ds scripts       # See available project scripts
```
