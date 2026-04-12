/// Generate the Fish initialization script
pub fn init_script() -> String {
    r#"# DeftShell Fish Integration
# Add to your ~/.config/fish/config.fish: ds init fish | source

# ── Guard against double-loading ──────────────────────────
if set -q _DEFTSHELL_LOADED
    return
end
set -gx _DEFTSHELL_LOADED 1
set -gx DEFTSHELL_SHELL "fish"

# ── Variables ─────────────────────────────────────────────
set -g _deftshell_cmd_start 0
set -g _deftshell_cmd_duration 0
set -g _deftshell_last_exit 0
set -g _deftshell_prev_dir $PWD

# ── Preexec: runs before every command ────────────────────
function _deftshell_preexec --on-event fish_preexec
    set -g _deftshell_cmd_start (date +%s%3N)

    # Safety check
    if command -q ds
        set -l safety_result (ds safety-check "$argv" 2>/dev/null)
        if test $status -ne 0 -a -n "$safety_result"
            echo $safety_result
        end
    end
end

# ── Postexec: runs after every command ────────────────────
function _deftshell_postexec --on-event fish_postexec
    # $status inside fish_postexec reflects the previous command's exit code
    # but only if captured first before any other command runs.
    set -g _deftshell_last_exit $status
    set -l cmd_end (date +%s%3N)

    if test $_deftshell_cmd_start -gt 0
        set -g _deftshell_cmd_duration (math "$cmd_end - $_deftshell_cmd_start")
    else
        set -g _deftshell_cmd_duration 0
    end
    set -g _deftshell_cmd_start 0

    # Track command and show suggestions (typos, aliases, sequences).
    if command -q ds
        ds track-command --command "$argv" --exit-code $_deftshell_last_exit --duration $_deftshell_cmd_duration --dir $PWD 1>/dev/null
    end
end

# ── Prompt ────────────────────────────────────────────────
function fish_prompt
    # Use the exit code captured in postexec rather than $status here,
    # which would reflect the last command in fish_postexec itself.
    set -l exit_code $_deftshell_last_exit
    if command -q ds
        set -l prompt_output (ds prompt-segment --shell fish --exit-code $exit_code --duration $_deftshell_cmd_duration 2>/dev/null)
        if test -n "$prompt_output"
            echo -n $prompt_output
            return
        end
    end
    # Fallback prompt
    set_color magenta; echo -n "ds "
    set_color cyan; echo -n (prompt_pwd)
    set_color normal; echo -n " > "
end

function fish_right_prompt
    set -l exit_code $status
    if command -q ds
        ds prompt-segment --shell fish --right --exit-code $exit_code 2>/dev/null
    end
end

# ── Directory change detection ────────────────────────────
function _deftshell_chpwd --on-variable PWD
    if test "$PWD" != "$_deftshell_prev_dir"
        set -g _deftshell_prev_dir $PWD
        if command -q ds
            ds context --detect --quiet &>/dev/null &
        end
    end
end

# ── Shell completions ─────────────────────────────────────
if command -q ds
    ds completions fish 2>/dev/null | source
end

# ── Context-aware aliases ─────────────────────────────────
if command -q ds
    ds alias --export --shell fish 2>/dev/null | source
end

# ── Initial context detection ─────────────────────────────
_deftshell_chpwd
"#
    .to_string()
}
