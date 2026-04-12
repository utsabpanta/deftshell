# Smart Prompt

DeftShell replaces your shell prompt with a context-aware prompt that shows git status, stack info, execution time, and more.

## Prompt Layout

```
✓ ~/projects/my-app (main*) [next] dev 1.2s >
│  │                 │       │      │   │
│  │                 │       │      │   └─ Last command duration (if >2s)
│  │                 │       │      └─ Environment (dev/staging/prod)
│  │                 │       └─ Detected framework
│  │                 └─ Git branch (* = dirty)
│  └─ Current directory
└─ Exit code (✓ = success, ✗ = failure)
```

## Themes

4 themes are available:

| Theme | Description |
|-------|-------------|
| `default` | Full info with colors |
| `minimal` | Compact, essential info only |
| `powerline` | Powerline-style with arrow separators |
| `pure` | Clean, minimal async prompt |

Set the theme in your config:

```toml
[prompt]
theme = "default"    # "default" | "minimal" | "powerline" | "pure"
```

## Configuration

```toml
[prompt]
theme = "default"
show_git = true                    # Show git branch and status
show_stack = true                  # Show detected framework
show_env = true                    # Show environment (dev/staging/prod)
show_execution_time = true         # Show last command duration
execution_time_threshold_ms = 2000 # Only show if command took longer than this
transient_prompt = true            # Collapse previous prompts
right_prompt = true                # Show right-aligned info
```

## Implementation

The prompt renderer uses the `git2` crate to read branch, dirty state, ahead/behind counts, and stash without shelling out to `git`. This keeps the prompt fast even in large repositories.
