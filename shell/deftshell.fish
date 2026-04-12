#!/usr/bin/env fish
# DeftShell Fish Integration
# Source this file in your ~/.config/fish/config.fish:
#   ds init fish | source
# Or source directly:
#   source /path/to/deftshell.fish

if command -q ds
    ds init fish | source
else
    echo "DeftShell (ds) not found in PATH. Install it first." >&2
end
