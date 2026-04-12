/// Generate the Bash initialization script
pub fn init_script() -> String {
    r#"# DeftShell Bash Integration
# Add to your ~/.bashrc: eval "$(ds init bash)"

# ── Guard against double-loading ──────────────────────────
[[ -n "$_DEFTSHELL_LOADED" ]] && return
export _DEFTSHELL_LOADED=1

# ── Configuration ─────────────────────────────────────────
export DEFTSHELL_SHELL="bash"

# ── Track previous directory for chpwd emulation ──────────
_DEFTSHELL_PREV_DIR="$PWD"
_DEFTSHELL_CMD_START=""
_DEFTSHELL_CMD_DURATION=0

# ── Preexec emulation for Bash ────────────────────────────
_DEFTSHELL_PREEXEC_READY=0
_deftshell_preexec() {
    # Only trigger on actual user commands, not prompt generation or
    # internal function calls. The PROMPT_COMMAND sets _PREEXEC_READY=1
    # so the DEBUG trap only fires once per user command.
    if [[ "$_DEFTSHELL_PREEXEC_READY" != "1" ]]; then
        return
    fi
    if [[ -n "$COMP_LINE" ]]; then
        return
    fi
    _DEFTSHELL_PREEXEC_READY=0
    _DEFTSHELL_CMD_START=$SECONDS
    local cmd="$(HISTTIMEFORMAT= history 1 | sed 's/^ *[0-9]* *//')"

    # Safety check
    if command -v ds &>/dev/null && [[ -n "$cmd" ]]; then
        local safety_result
        safety_result="$(ds safety-check "$cmd" 2>/dev/null)"
        if [[ $? -ne 0 && -n "$safety_result" ]]; then
            echo "$safety_result"
        fi
    fi
}
trap '_deftshell_preexec' DEBUG

# ── Prompt command: runs before every prompt ──────────────
_deftshell_prompt_command() {
    local exit_code=$?

    # Calculate command duration
    if [[ -n "$_DEFTSHELL_CMD_START" ]]; then
        _DEFTSHELL_CMD_DURATION=$(( (SECONDS - _DEFTSHELL_CMD_START) * 1000 ))
    else
        _DEFTSHELL_CMD_DURATION=0
    fi
    _DEFTSHELL_CMD_START=""

    # Check for directory change (chpwd emulation)
    if [[ "$PWD" != "$_DEFTSHELL_PREV_DIR" ]]; then
        _DEFTSHELL_PREV_DIR="$PWD"
        if command -v ds &>/dev/null; then
            ds context --detect --quiet &>/dev/null &
        fi
    fi

    # Track command and show suggestions (typos, aliases, sequences).
    local cmd="$(HISTTIMEFORMAT= history 1 | sed 's/^ *[0-9]* *//')"
    if command -v ds &>/dev/null && [[ -n "$cmd" ]]; then
        ds track-command --command "$cmd" --exit-code $exit_code --duration $_DEFTSHELL_CMD_DURATION --dir "$PWD" 2>&2 1>/dev/null
    fi

    # Update prompt
    if command -v ds &>/dev/null; then
        PS1="$(ds prompt-segment --shell bash --exit-code $exit_code --duration $_DEFTSHELL_CMD_DURATION 2>/dev/null)"
        if [[ -z "$PS1" ]]; then
            PS1="\[\033[35m\]ds\[\033[0m\] \[\033[36m\]\w\[\033[0m\] \$ "
        fi
    fi

    # Re-arm the preexec guard so the DEBUG trap fires for the next user command.
    _DEFTSHELL_PREEXEC_READY=1
}

PROMPT_COMMAND="_deftshell_prompt_command${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

# ── Shell completions ─────────────────────────────────────
if command -v ds &>/dev/null; then
    eval "$(ds completions bash 2>/dev/null)"
fi

# ── Context-aware aliases ─────────────────────────────────
if command -v ds &>/dev/null; then
    eval "$(ds alias --export --shell bash 2>/dev/null)"
fi

# ── Initial context detection ─────────────────────────────
if command -v ds &>/dev/null; then
    ds context --detect --quiet &>/dev/null &
fi
"#
    .to_string()
}
