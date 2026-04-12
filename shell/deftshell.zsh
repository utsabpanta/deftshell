#!/usr/bin/env zsh
# DeftShell Zsh Integration
# Source this file in your ~/.zshrc:
#   eval "$(ds init zsh)"
# Or source directly:
#   source /path/to/deftshell.zsh

# The actual init script is generated dynamically by `ds init zsh`
# to include the correct binary path. This file serves as a reference
# and fallback.

if command -v ds &>/dev/null; then
    eval "$(ds init zsh)"
else
    echo "DeftShell (ds) not found in PATH. Install it first." >&2
fi
