# Security Rules

- Credentials are stored in `~/.deftshell/credentials.toml` with 0600 permissions
- Never log, print, or include API keys in error messages
- All SQL queries must use parameterized statements (`params![]`), never string concatenation
- Plugin names must be validated before use in filesystem paths (no `..`, `/`, `\`)
- The database file must have 0600 permissions on Unix
- Sensitive command arguments are redacted before storing in command_history (see tracker.rs)
- AI context builder must exclude .env files by default
- All HTTP requests to external APIs must use HTTPS (except localhost Ollama)
- Shell commands from AI responses must pass through the safety interceptor before execution
- User must explicitly confirm before executing any AI-generated command
