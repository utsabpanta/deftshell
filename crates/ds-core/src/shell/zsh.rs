/// Generate the Zsh initialization script
pub fn init_script() -> String {
    r#"# DeftShell Zsh Integration
# Add to your ~/.zshrc: eval "$(ds init zsh)"

# ── Clean up previous load (allows safe re-sourcing) ─────
if [[ -n "$_DEFTSHELL_LOADED" ]]; then
    autoload -Uz add-zsh-hook
    add-zsh-hook -d precmd  _deftshell_precmd_timer 2>/dev/null
    add-zsh-hook -d precmd  _deftshell_precmd       2>/dev/null
    add-zsh-hook -d preexec _deftshell_preexec       2>/dev/null
    add-zsh-hook -d preexec _deftshell_preexec_timer 2>/dev/null
    add-zsh-hook -d chpwd   _deftshell_chpwd         2>/dev/null
fi
export _DEFTSHELL_LOADED=1

# ── Configuration ─────────────────────────────────────────
export DEFTSHELL_SHELL="zsh"

# ── Precmd hook: runs before every prompt ─────────────────
_deftshell_precmd() {
    # _DEFTSHELL_LAST_EXIT is already captured by _deftshell_precmd_timer
    # (which runs first) so that $? isn't clobbered by intermediate commands.
    local cmd_duration=${_DEFTSHELL_CMD_DURATION:-0}

    # Track the previous command and show suggestions (typos, aliases, sequences).
    if command -v ds &>/dev/null && [[ -n "$_DEFTSHELL_LAST_CMD" ]]; then
        ds track-command --command "$_DEFTSHELL_LAST_CMD" --exit-code $_DEFTSHELL_LAST_EXIT --duration $cmd_duration --dir "$PWD" 1>/dev/null
    fi
    unset _DEFTSHELL_LAST_CMD

    # Update prompt with context
    if command -v ds &>/dev/null; then
        PROMPT="$(ds prompt-segment --shell zsh --exit-code $_DEFTSHELL_LAST_EXIT --duration $cmd_duration 2>/dev/null)"
        if [[ -z "$PROMPT" ]]; then
            PROMPT="%F{magenta}ds%f %F{cyan}%~%f %# "
        fi

        # Right prompt
        RPROMPT="$(ds prompt-segment --shell zsh --right --exit-code $_DEFTSHELL_LAST_EXIT 2>/dev/null)"
    fi

    unset _DEFTSHELL_CMD_START
    unset _DEFTSHELL_CMD_DURATION
}

# ── Preexec hook: runs before every command ───────────────
_deftshell_preexec() {
    _DEFTSHELL_CMD_START=$EPOCHREALTIME
    # Zsh preexec receives the command string as $1.
    _DEFTSHELL_LAST_CMD="$1"

    # Safety check — warn about dangerous commands
    if command -v ds &>/dev/null; then
        local safety_result
        safety_result="$(ds safety-check "$1" 2>/dev/null)"
        if [[ $? -ne 0 && -n "$safety_result" ]]; then
            echo "$safety_result"
        fi
    fi
}

# ── Chpwd hook: runs on directory change ──────────────────
_deftshell_chpwd() {
    if command -v ds &>/dev/null; then
        # Trigger context detection in background
        ds context --detect --quiet &>/dev/null &!
    fi
}

# ── Command duration tracking ─────────────────────────────
_deftshell_preexec_timer() {
    _DEFTSHELL_CMD_START=${EPOCHREALTIME:-$(date +%s)}
}

_deftshell_precmd_timer() {
    # Capture exit code FIRST — this hook runs before _deftshell_precmd,
    # so $? still reflects the user's command here.
    _DEFTSHELL_LAST_EXIT=$?

    if [[ -n "$_DEFTSHELL_CMD_START" ]]; then
        local end=${EPOCHREALTIME:-$(date +%s)}
        _DEFTSHELL_CMD_DURATION=$(( (end - _DEFTSHELL_CMD_START) * 1000 ))
        _DEFTSHELL_CMD_DURATION=${_DEFTSHELL_CMD_DURATION%.*}
    else
        _DEFTSHELL_CMD_DURATION=0
    fi
}

# ── Register hooks ────────────────────────────────────────
autoload -Uz add-zsh-hook

add-zsh-hook precmd _deftshell_precmd_timer
add-zsh-hook precmd _deftshell_precmd
add-zsh-hook preexec _deftshell_preexec
add-zsh-hook preexec _deftshell_preexec_timer
add-zsh-hook chpwd _deftshell_chpwd

# ── Shell completions ─────────────────────────────────────
if command -v ds &>/dev/null; then
    eval "$(ds completions zsh 2>/dev/null)"
fi

# ── Context-aware aliases ─────────────────────────────────
_deftshell_load_aliases() {
    if command -v ds &>/dev/null; then
        eval "$(ds alias --export --shell zsh 2>/dev/null)"
    fi
}
_deftshell_load_aliases

# ── Initial context detection ─────────────────────────────
_deftshell_chpwd
"#
    .to_string()
}
