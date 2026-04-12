# Installation

## From Source (Rust)

```bash
cargo install --git https://github.com/utsabpanta/deftshell.git ds-cli
```

## Homebrew (macOS)

```bash
brew tap utsabpanta/deftshell
brew install deftshell
```

## Direct Download

```bash
curl -fsSL https://raw.githubusercontent.com/utsabpanta/deftshell/main/scripts/install.sh | bash
```

## Verify Installation

After installing, verify everything is working:

```bash
ds doctor
```

This runs diagnostics and shows the status of all components:

```
DeftShell Doctor
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  [OK] ds binary in PATH
  [OK] Shell integration configured
  [OK] Git available
  [OK] Node.js available

AI Providers:
  [OK]   Ollama (local)
  [OK]   Anthropic API key
  [!!]   OpenAI API key (not set)

Storage:
  [OK] Database accessible
  [OK] Data directory exists
  [OK] User config file

  8 passed, 1 warning, 0 failed
```

## Next Steps

- [Set up shell integration](shell-setup.md)
- [Configure an AI provider](../guides/ai-providers.md)
