#!/bin/sh
# Safety hook: blocks dangerous commands before they execute.
# Exit 0 = allow, exit 2 = block.
# Reads tool input JSON from stdin (piped by Claude Code).

# Use printf instead of echo to avoid shell interpretation of command content.
COMMAND=$(jq -r '.tool_input.command // empty' 2>/dev/null)

# If jq failed or command is empty, allow (don't break the workflow).
if [ -z "$COMMAND" ]; then
  exit 0
fi

# Write command to a temp file for safe grep (avoids shell interpretation).
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT
printf '%s\n' "$COMMAND" > "$TMPFILE"

# Block production-dangerous patterns
if grep -qiE 'rm\s+-rf\s+/|rm\s+-rf\s+~|cargo\s+publish|git\s+push.*--force\s+origin\s+(main|master|production|release)' "$TMPFILE"; then
  echo "BLOCKED: dangerous command pattern detected" >&2
  exit 2
fi

# Block accidental secret leaks
if grep -qiE 'echo.*(API_KEY|SECRET|PASSWORD|TOKEN)|cat.*credentials' "$TMPFILE"; then
  echo "BLOCKED: potential secret leak detected" >&2
  exit 2
fi

exit 0
