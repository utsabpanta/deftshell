#!/usr/bin/env bash
# DeftShell Bash Integration
# Source this file in your ~/.bashrc:
#   eval "$(ds init bash)"
# Or source directly:
#   source /path/to/deftshell.bash

if command -v ds &>/dev/null; then
    eval "$(ds init bash)"
else
    echo "DeftShell (ds) not found in PATH. Install it first." >&2
fi
