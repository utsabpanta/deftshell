# Safety Engine

DeftShell intercepts dangerous commands before execution, protecting against accidental data loss and destructive operations.

## How It Works

When you run a command, DeftShell checks it against built-in and custom safety rules before passing it to the shell. If a rule matches, you'll see a warning with the risk level and an option to proceed or cancel.

```
$ rm -rf /
  ⚠ CAUTION: Destructive Command Detected
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Command:  rm -rf /
  Risk:     CRITICAL
  Reason:   Recursive forced deletion of the root filesystem

  Suggestion: Specify the exact directory you want to remove instead of /
```

## Risk Levels

| Level | Description | Default Action |
|-------|-------------|----------------|
| **Critical** | Potentially catastrophic (data loss, system damage) | Block and require explicit confirmation |
| **High** | Significant risk of data loss or unintended changes | Warn and ask for confirmation |
| **Medium** | Moderate risk, usually recoverable | Show warning |
| **Low** | Minor risk, informational | Log only |

## Built-in Rules

DeftShell includes 29 built-in rules across 4 risk levels:

| Level | Count | Examples |
|-------|-------|---------|
| Critical | 11 | `rm -rf /`, `rm -rf ~`, `chmod 777 /`, `dd if=/dev/zero`, `mkfs`, fork bombs, `curl\|sh` |
| High | 12 | `git push --force`, `git reset --hard`, `DROP TABLE`, `docker system prune -a`, `terraform destroy`, `kubectl delete namespace` |
| Medium | 6 | `rm -rf node_modules`, `git checkout -- .`, `chmod -R`, `chown -R` |
| Low | 0 | Reserved for custom rules |

## Three-Layer Architecture

### Layer 1 — Rules

Compiled regex patterns that match dangerous commands. Example for detecting `rm -rf /`:

```regex
rm\s+(-[a-zA-Z]*r[a-zA-Z]*\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?|
      (-[a-zA-Z]*f[a-zA-Z]*\s+)?-[a-zA-Z]*r[a-zA-Z]*\s+)/\s*$
```

This matches `rm -rf /`, `rm -fr /`, `rm -r -f /`, etc.

### Layer 2 — Interceptor

Orchestrates the check:
1. If safety disabled → pass
2. Check allowlist (configurable patterns that always pass)
3. Check denylist (configurable patterns that always block as CRITICAL)
4. Match against all rules, return the highest-severity match

### Layer 3 — Risk Assessor

Context-aware elevation that can only **increase** risk, never decrease:

| Context | Condition | Effect |
|---------|-----------|--------|
| Protected branch | On `main`/`master`/`production`/`prod`/`release` | Git + DB ops elevated 1 level |
| Production env | `is_production_env` flag | DB + infra ops elevated 1 level |
| K8s production | Context contains "prod"/"production"/"prd"/"live" | Everything → CRITICAL |
| Uncommitted changes | `has_uncommitted_changes` | Destructive git ops elevated 1 level |

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
