# Command Intelligence

DeftShell learns from your command history to detect typos, suggest aliases, and identify command sequences.

## Typo Detection

DeftShell automatically detects command typos and suggests corrections:

```
$ gti status
Did you mean `git status`?
```

Uses two algorithms together:

1. **Fuzzy matching** (Skim algorithm) — scores how similar the typed command is to known commands
2. **Levenshtein edit distance** — counts minimum single-character edits to transform one string into another

A command is flagged as a typo when: `edit_distance <= 2 AND edit_distance > 0 AND fuzzy_score > 10`

Checked against 30 common commands including: `git`, `docker`, `npm`, `yarn`, `pnpm`, `cargo`, `python`, `pip`, `node`, `npx`, `kubectl`, `terraform`, `make`, `curl`, `wget`, `ssh`, and more.

## Alias Suggestions

If you run the same command 20+ times and it's longer than 15 characters, DeftShell suggests creating an alias:

```
You've run "npm run test" 23 times. Create an alias?
  ds alias add nrt="npm run test"
```

The alias name is auto-generated from the first letter of each word (up to 4).

## Sequence Detection

DeftShell analyzes your last 100 commands per directory. If command B follows command A at least 5 times, it suggests running B automatically after A:

```
You often run "cargo test" after "cargo build". Run it now?
```

## Command Tracking

Every command is recorded to SQLite via the shell `precmd` hook, with **sensitive data redacted** before storage:

- `PASSWORD=secret123` → `PASSWORD=***`
- `Bearer abc123` → `Bearer ***`
- `-p mypassword` → `-p ***`

## Analytics

View your command usage statistics:

```bash
ds stats                    # Project stats
ds stats today              # Today's usage
ds stats week               # This week
ds stats --format json      # Export as JSON
```
