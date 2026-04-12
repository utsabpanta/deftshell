# Safety Engine

DeftShell's safety engine intercepts dangerous commands before execution, protecting against accidental data loss and destructive operations.

## How It Works

When you run a command, DeftShell checks it against built-in and custom safety rules before passing it to the shell. If a rule matches, you'll see a warning with the risk level and an option to proceed or cancel.

## Risk Levels

| Level | Description | Default Action |
|-------|-------------|----------------|
| **Critical** | Potentially catastrophic (data loss, system damage) | Block and require explicit confirmation |
| **High** | Significant risk of data loss or unintended changes | Warn and ask for confirmation |
| **Medium** | Moderate risk, usually recoverable | Show warning |
| **Low** | Minor risk, informational | Log only |

## Built-in Rules

DeftShell includes 20+ built-in rules covering:

- **File system**: `rm -rf /`, `chmod 777`, recursive deletions
- **Git**: `git push --force`, `git reset --hard`, branch deletion
- **Docker**: `docker system prune`, volume removal
- **Database**: `DROP TABLE`, `DELETE FROM` without WHERE, `TRUNCATE`
- **Kubernetes**: `kubectl delete namespace`, production deployments
- **System**: Fork bombs, disk formatting, permission changes
- **Infrastructure**: `terraform destroy`, production deployments

## Context-Aware Elevation

The safety engine considers your current context to elevate risk levels:

- **Production environment** — All risks elevated
- **Protected git branch** (main, master, production) — Git operations elevated
- **Uncommitted changes** — Destructive git operations elevated
- **Kubernetes production context** — All K8s operations elevated to critical

## Configuration

```toml
[safety]
enabled = true
confirm_threshold = "medium"    # Show confirmation for this level and above
require_confirmation = true     # Require explicit yes/no for dangerous commands

# Commands that bypass safety checks
allowlist = ["git push origin main"]

# Commands that are always blocked
denylist = ["rm -rf /"]
```

## Custom Rules

Add project-specific safety rules in `.deftshell.toml`:

```toml
[[safety.custom_rules.rule]]
pattern = "prisma migrate reset"
level = "high"
message = "This will reset the entire database. Make sure you're not in production."
suggestion = "Use 'prisma migrate dev' for development migrations"

[[safety.custom_rules.rule]]
pattern = "npm publish"
level = "medium"
message = "Publishing to npm. Verify the version number first."
```

## Disabling Safety

For automated scripts, you can disable safety checks:

```toml
[safety]
enabled = false
```

Or bypass for specific commands via the allowlist.
