use anyhow::{bail, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use colored::Colorize;
use ds_core::config::DeftShellConfig;

use crate::Cli;

/// `ds init <shell>` - output shell integration script
pub fn run(shell: &str) -> Result<()> {
    let shell_type: ds_core::config::ShellType = shell.parse()?;
    let script = ds_core::shell::generate_init_script(shell_type);
    print!("{}", script);
    Ok(())
}

/// `ds version`
pub fn run_version() -> Result<()> {
    println!("DeftShell (ds) v{}", env!("CARGO_PKG_VERSION"));
    println!(
        "  Shell: {}",
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".into())
    );
    println!("  OS: {}", std::env::consts::OS);
    println!("  Arch: {}", std::env::consts::ARCH);
    Ok(())
}

/// `ds completions <shell>` - generate real shell completions using clap_complete
pub fn run_completions(shell: &str) -> Result<()> {
    let shell_variant = match shell.to_lowercase().as_str() {
        "zsh" => Shell::Zsh,
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "elvish" => Shell::Elvish,
        "powershell" | "ps" => Shell::PowerShell,
        other => bail!(
            "Unsupported shell '{}'. Supported shells: zsh, bash, fish, elvish, powershell",
            other
        ),
    };

    let mut cmd = Cli::command();
    let bin_name = "ds".to_string();
    generate(shell_variant, &mut cmd, bin_name, &mut std::io::stdout());

    Ok(())
}

/// `ds update` - self-update with version check against GitHub releases
pub fn run_update(channel: Option<&str>, check: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let channel = channel.unwrap_or("stable");

    println!("{}", "DeftShell Update".bold());
    println!(
        "{}",
        "---------------------------------------------".dimmed()
    );
    println!(
        "  Current version:  {}",
        format!("v{}", current_version).cyan()
    );
    println!("  Channel:          {}", channel.yellow());
    println!();

    // Determine the GitHub API URL based on channel
    let api_url = match channel {
        "stable" => {
            "https://api.github.com/repos/deftshell-io/deftshell/releases/latest".to_string()
        }
        "beta" | "nightly" => {
            // For beta/nightly, list releases and find the latest pre-release
            "https://api.github.com/repos/deftshell-io/deftshell/releases".to_string()
        }
        other => {
            bail!(
                "Unknown update channel '{}'. Supported channels: stable, beta, nightly",
                other
            );
        }
    };

    println!("  Checking for updates...");

    // Attempt to fetch the latest release from GitHub
    match fetch_latest_version(&api_url, channel) {
        Ok(release_info) => {
            let latest = &release_info.version;
            println!("  Latest version:   {}", format!("v{}", latest).cyan());
            println!();

            match version_cmp(current_version, latest) {
                std::cmp::Ordering::Less => {
                    println!(
                        "  {} Update available: v{} -> v{}",
                        ">>>".green().bold(),
                        current_version,
                        latest
                    );

                    if check {
                        println!();
                        println!("  Run {} to install the update.", "ds update".bold());
                    } else {
                        println!();
                        install_update(&release_info)?;
                    }
                }
                std::cmp::Ordering::Equal => {
                    println!(
                        "  {} You are already on the latest version.",
                        "OK".green().bold()
                    );
                }
                std::cmp::Ordering::Greater => {
                    println!(
                        "  {} Your version is newer than the latest release (dev build?).",
                        "OK".green().bold()
                    );
                }
            }
        }
        Err(e) => {
            println!(
                "  {} Could not check for updates: {}",
                "WARN".yellow().bold(),
                e
            );
            println!();
            print_manual_update_instructions();
        }
    }

    Ok(())
}

/// Information about a GitHub release
struct ReleaseInfo {
    version: String,
    tag_name: String,
    html_url: String,
    #[allow(dead_code)]
    prerelease: bool,
}

/// Fetch the latest version information from GitHub Releases API.
///
/// Uses `tokio::task::block_in_place` to safely run blocking HTTP requests
/// from within the tokio async runtime (main is `#[tokio::main]`).
fn fetch_latest_version(api_url: &str, channel: &str) -> Result<ReleaseInfo> {
    // We are called from a sync function inside a tokio runtime.
    // `reqwest::blocking` will panic if used directly inside an async runtime,
    // so we use `block_in_place` to move off the async worker thread.
    let api_url = api_url.to_string();
    let channel = channel.to_string();

    tokio::task::block_in_place(move || fetch_latest_version_blocking(&api_url, &channel))
}

/// Inner blocking implementation for the GitHub API fetch.
fn fetch_latest_version_blocking(api_url: &str, channel: &str) -> Result<ReleaseInfo> {
    // Build a blocking HTTP client with a reasonable timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("deftshell/{}", env!("CARGO_PKG_VERSION")))
        .build()?;

    if channel == "stable" {
        // The /releases/latest endpoint returns a single release object
        let resp = client.get(api_url).send()?;

        if !resp.status().is_success() {
            bail!(
                "GitHub API returned status {} ({})",
                resp.status().as_u16(),
                resp.status().canonical_reason().unwrap_or("unknown")
            );
        }

        let body: serde_json::Value = resp.json()?;
        let tag_name = body["tag_name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tag_name in release response"))?
            .to_string();
        let version = tag_name.strip_prefix('v').unwrap_or(&tag_name).to_string();
        let html_url = body["html_url"].as_str().unwrap_or("").to_string();
        let prerelease = body["prerelease"].as_bool().unwrap_or(false);

        Ok(ReleaseInfo {
            version,
            tag_name,
            html_url,
            prerelease,
        })
    } else {
        // For beta/nightly, fetch the list and pick the first pre-release
        let resp = client.get(api_url).send()?;

        if !resp.status().is_success() {
            bail!(
                "GitHub API returned status {} ({})",
                resp.status().as_u16(),
                resp.status().canonical_reason().unwrap_or("unknown")
            );
        }

        let body: serde_json::Value = resp.json()?;
        let releases = body
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Expected array of releases"))?;

        // Find the first pre-release for beta, or any release for nightly
        let release = releases
            .iter()
            .find(|r| {
                let is_pre = r["prerelease"].as_bool().unwrap_or(false);
                match channel {
                    "beta" => is_pre,
                    "nightly" => true, // nightly takes the very latest regardless
                    _ => false,
                }
            })
            .ok_or_else(|| anyhow::anyhow!("No {} releases found", channel))?;

        let tag_name = release["tag_name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tag_name in release"))?
            .to_string();
        let version = tag_name.strip_prefix('v').unwrap_or(&tag_name).to_string();
        let html_url = release["html_url"].as_str().unwrap_or("").to_string();
        let prerelease = release["prerelease"].as_bool().unwrap_or(false);

        Ok(ReleaseInfo {
            version,
            tag_name,
            html_url,
            prerelease,
        })
    }
}

/// Compare two semver-like version strings.
/// Returns Ordering::Less if a < b, Equal if a == b, Greater if a > b.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|part| {
                // Strip any pre-release suffix for numeric comparison
                let numeric: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
                numeric.parse::<u64>().unwrap_or(0)
            })
            .collect()
    };

    let va = parse(a);
    let vb = parse(b);

    let max_len = va.len().max(vb.len());
    for i in 0..max_len {
        let pa = va.get(i).copied().unwrap_or(0);
        let pb = vb.get(i).copied().unwrap_or(0);
        match pa.cmp(&pb) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Attempt to perform the actual binary update.
fn install_update(release: &ReleaseInfo) -> Result<()> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Map Rust target triples to expected asset naming conventions
    let platform = match (os, arch) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => {
            println!(
                "  {} Automatic update is not supported for {}/{}.",
                "WARN".yellow().bold(),
                os,
                arch
            );
            println!();
            print_manual_update_instructions();
            return Ok(());
        }
    };

    let expected_asset = format!("ds-{}.tar.gz", platform);

    println!(
        "  Platform:         {} ({})",
        platform.dimmed(),
        expected_asset.dimmed()
    );
    println!("  Release:          {}", release.html_url.dimmed());
    println!();
    println!(
        "  {} To complete the update, download the release binary:",
        "NOTE".cyan().bold()
    );
    println!();
    println!(
        "    {}",
        format!(
            "curl -fsSL https://github.com/deftshell-io/deftshell/releases/download/{}/{} | tar xz",
            release.tag_name, expected_asset
        )
        .bold()
    );
    println!();
    println!("  Or use one of the methods below:");
    println!();
    print_manual_update_instructions();

    Ok(())
}

/// Print manual update instructions for various installation methods.
fn print_manual_update_instructions() {
    println!("  Install / update methods:");
    println!();
    println!("    {}    cargo install ds-cli", "Cargo:".bold());
    println!("    {}  brew upgrade deftshell", "Homebrew:".bold());
    println!(
        "    {}   Download from https://github.com/deftshell-io/deftshell/releases",
        "GitHub:".bold()
    );
}

/// `ds help [command]` - contextual help system with grouped commands
pub fn run_help(command: Option<&str>) -> Result<()> {
    match command {
        Some(cmd) => show_command_help(cmd),
        None => show_overview_help(),
    }
}

/// Show a high-level overview of all commands, grouped by category.
fn show_overview_help() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");

    println!();
    println!("  {}  v{}", "DeftShell".bold().cyan(), version);
    println!(
        "  {}",
        "The AI-Powered Context-Aware Terminal for Developers".dimmed()
    );
    println!();
    println!("  {}", "USAGE:".bold().underline());
    println!("    ds <COMMAND> [OPTIONS]");
    println!();

    // AI Commands
    print_category("AI Commands");
    print_command("ask <query>", "Ask AI a question with project context");
    print_command("do <instruction>", "AI generates and executes commands");
    print_command("how <question>", "Get project-aware how-to instructions");
    print_command("explain", "Explain piped command output");
    print_command("review", "Review piped code changes");
    print_command("chat", "Interactive AI chat mode");
    print_command("generate <type> [name]", "AI code generation");
    println!();

    // Context & Project Commands
    print_category("Context & Project");
    print_command("context", "Show detected project context");
    print_command("env", "Environment context display");
    print_command("scripts", "List project scripts");
    print_command("run <script>", "Run a project script");
    print_command("workspace", "Monorepo workspace commands");
    println!();

    // Automation Commands
    print_category("Automation");
    print_command(
        "runbook <action>",
        "Runbook management (create, run, share)",
    );
    print_command("alias", "Manage context-aware aliases");
    println!();

    // Plugin & Extension Commands
    print_category("Plugins & Extensions");
    print_command(
        "plugin <action>",
        "Plugin management (install, remove, update)",
    );
    println!();

    // Configuration & Auth Commands
    print_category("Configuration & Auth");
    print_command("config", "Configuration management");
    print_command("auth <action>", "AI provider authentication");
    print_command("privacy <on|off>", "Privacy controls");
    println!();

    // Diagnostics & Info Commands
    print_category("Diagnostics & Info");
    print_command("stats [period]", "Analytics dashboard");
    print_command("usage", "AI usage and cost tracking");
    print_command("doctor", "Diagnose issues");
    print_command("version", "Version information");
    println!();

    // Setup Commands
    print_category("Setup");
    print_command("init <shell>", "Generate shell initialization script");
    print_command("completions <shell>", "Generate shell completions");
    print_command("update", "Self-update DeftShell");
    println!();

    // Global options
    println!("  {}", "GLOBAL OPTIONS:".bold().underline());
    print_option("--provider <name>", "Override AI provider for this command");
    print_option("-y, --yes", "Auto-confirm all prompts");
    print_option("--dry-run", "Show what would be done without executing");
    print_option("--output <file>", "Write output to a file");
    println!();

    println!(
        "  Run {} for more information about a specific command.",
        "ds help <command>".bold()
    );
    println!();

    Ok(())
}

/// Show detailed help for a specific command.
fn show_command_help(cmd: &str) -> Result<()> {
    // Build the full command help database
    let help = get_command_help(cmd);

    match help {
        Some(entry) => {
            println!();
            println!("  {} {}", "ds".bold(), cmd.bold().cyan());
            println!(
                "  {}",
                "---------------------------------------------".dimmed()
            );
            println!("  {}", entry.description);
            println!();

            println!("  {}", "USAGE:".bold().underline());
            println!("    {}", entry.usage);
            println!();

            if !entry.examples.is_empty() {
                println!("  {}", "EXAMPLES:".bold().underline());
                for example in &entry.examples {
                    println!("    {}", example.dimmed());
                }
                println!();
            }

            if !entry.flags.is_empty() {
                println!("  {}", "OPTIONS:".bold().underline());
                for (flag, desc) in &entry.flags {
                    println!("    {:<28} {}", flag.bold(), desc);
                }
                println!();
            }

            if !entry.subcommands.is_empty() {
                println!("  {}", "SUBCOMMANDS:".bold().underline());
                for (sub, desc) in &entry.subcommands {
                    println!("    {:<28} {}", sub.bold(), desc);
                }
                println!();
            }

            if let Some(note) = &entry.note {
                println!("  {} {}", "NOTE:".yellow().bold(), note);
                println!();
            }
        }
        None => {
            println!();
            println!("  {} Unknown command '{}'.", "ERROR:".red().bold(), cmd);
            println!("  Run {} to see all available commands.", "ds help".bold());
            println!();
        }
    }

    Ok(())
}

struct CommandHelp {
    description: &'static str,
    usage: &'static str,
    examples: Vec<&'static str>,
    flags: Vec<(&'static str, &'static str)>,
    subcommands: Vec<(&'static str, &'static str)>,
    note: Option<&'static str>,
}

fn get_command_help(cmd: &str) -> Option<CommandHelp> {
    match cmd {
        "ask" => Some(CommandHelp {
            description: "Ask the AI a question with full project context automatically included.",
            usage: "ds ask <query>...",
            examples: vec![
                "ds ask how do I run the tests",
                "ds ask what does the UserService class do",
                "ds ask --provider openai explain the database schema",
            ],
            flags: vec![
                ("--provider <name>", "Override the AI provider"),
            ],
            subcommands: vec![],
            note: Some("Project context (language, framework, dependencies) is auto-detected."),
        }),
        "do" => Some(CommandHelp {
            description: "Give a natural language instruction and the AI will generate and execute the appropriate shell commands.",
            usage: "ds do <instruction>...",
            examples: vec![
                "ds do create a new React component called Button",
                "ds do run database migrations",
                "ds do find all TODO comments in the codebase",
                "ds do --dry-run deploy to staging",
            ],
            flags: vec![
                ("-y, --yes", "Auto-confirm command execution"),
                ("--dry-run", "Show commands without executing"),
            ],
            subcommands: vec![],
            note: Some("Commands are safety-checked before execution."),
        }),
        "how" => Some(CommandHelp {
            description: "Get step-by-step how-to instructions tailored to your project.",
            usage: "ds how <question>...",
            examples: vec![
                "ds how do I add a new API endpoint",
                "ds how to set up CI/CD",
                "ds how to configure logging",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("Answers are contextual to your detected project stack."),
        }),
        "explain" => Some(CommandHelp {
            description: "Pipe command output to ds explain for an AI-powered explanation.",
            usage: "command | ds explain",
            examples: vec![
                "cat error.log | ds explain",
                "docker logs my-app | ds explain",
                "cargo build 2>&1 | ds explain",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("Reads from stdin. Pipe the output you want explained."),
        }),
        "review" => Some(CommandHelp {
            description: "Pipe code diffs or changes for AI-powered code review.",
            usage: "command | ds review",
            examples: vec![
                "git diff | ds review",
                "git diff --staged | ds review",
                "git show HEAD | ds review",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("Reads from stdin. Pipe the diff you want reviewed."),
        }),
        "chat" => Some(CommandHelp {
            description: "Start an interactive AI chat session with project context.",
            usage: "ds chat [OPTIONS]",
            examples: vec![
                "ds chat",
                "ds chat --continue",
                "ds chat --context src/main.rs",
            ],
            flags: vec![
                ("--continue", "Resume the last conversation"),
                ("--context <file>", "Include a specific file in context"),
            ],
            subcommands: vec![],
            note: None,
        }),
        "generate" => Some(CommandHelp {
            description: "AI code generation for common patterns and boilerplate.",
            usage: "ds generate <type> [name]",
            examples: vec![
                "ds generate component Button",
                "ds generate migration add-users-table",
                "ds generate test UserService",
                "ds generate dockerfile",
                "ds generate github-action ci",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("Supported types: component, migration, test, dockerfile, github-action."),
        }),
        "context" => Some(CommandHelp {
            description: "Display and manage the auto-detected project context.",
            usage: "ds context [SUBCOMMAND] [OPTIONS]",
            examples: vec![
                "ds context",
                "ds context refresh",
                "ds context export",
                "ds context diff",
                "ds context --detect --quiet",
            ],
            flags: vec![
                ("--detect", "Detect context quietly (used by shell hooks)"),
                ("--quiet", "Suppress output"),
            ],
            subcommands: vec![
                ("refresh", "Force re-detection of project context"),
                ("export", "Export context as JSON"),
                ("diff", "Show changes since last detection"),
            ],
            note: None,
        }),
        "env" => Some(CommandHelp {
            description: "Display environment context including detected tools, runtimes, and project information.",
            usage: "ds env",
            examples: vec!["ds env"],
            flags: vec![],
            subcommands: vec![],
            note: None,
        }),
        "scripts" => Some(CommandHelp {
            description: "List all runnable scripts detected in the current project.",
            usage: "ds scripts",
            examples: vec!["ds scripts"],
            flags: vec![],
            subcommands: vec![],
            note: Some("Detects scripts from package.json, Makefile, Cargo.toml, etc."),
        }),
        "run" => Some(CommandHelp {
            description: "Run a detected project script by name.",
            usage: "ds run <script>",
            examples: vec![
                "ds run test",
                "ds run build",
                "ds run lint",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("Use 'ds scripts' to see available scripts."),
        }),
        "workspace" => Some(CommandHelp {
            description: "Monorepo workspace management commands.",
            usage: "ds workspace [SUBCOMMAND]",
            examples: vec!["ds workspace list"],
            flags: vec![],
            subcommands: vec![
                ("list", "List all packages in the monorepo"),
            ],
            note: None,
        }),
        "runbook" => Some(CommandHelp {
            description: "Create, manage, and execute runbooks -- reproducible multi-step command sequences.",
            usage: "ds runbook <SUBCOMMAND>",
            examples: vec![
                "ds runbook new deploy-staging",
                "ds runbook run deploy-staging",
                "ds runbook record",
                "ds runbook list",
                "ds runbook generate set up CI pipeline",
            ],
            flags: vec![],
            subcommands: vec![
                ("new <name>", "Create a new runbook"),
                ("edit <name>", "Edit a runbook"),
                ("delete <name>", "Delete a runbook"),
                ("list", "List all runbooks"),
                ("show <name>", "Display runbook steps"),
                ("run <name>", "Execute a runbook"),
                ("record [name]", "Start recording commands"),
                ("stop", "Stop recording"),
                ("generate <desc>", "AI-generate a runbook"),
                ("search <query>", "Search community registry"),
                ("install <spec>", "Install from registry"),
                ("publish <name>", "Publish to registry"),
                ("trending", "Show trending runbooks"),
            ],
            note: None,
        }),
        "alias" => Some(CommandHelp {
            description: "Manage context-aware shell aliases.",
            usage: "ds alias [SUBCOMMAND] [OPTIONS]",
            examples: vec![
                "ds alias list",
                "ds alias add gp='git push'",
                "ds alias remove gp",
                "ds alias --export --shell zsh",
            ],
            flags: vec![
                ("--export", "Export aliases for shell sourcing"),
                ("--shell <type>", "Shell type for export (zsh, bash, fish)"),
            ],
            subcommands: vec![
                ("add <spec>", "Add an alias (alias=command format)"),
                ("remove <name>", "Remove an alias"),
                ("list", "List all aliases"),
            ],
            note: None,
        }),
        "plugin" => Some(CommandHelp {
            description: "Manage the DeftShell plugin ecosystem.",
            usage: "ds plugin <SUBCOMMAND>",
            examples: vec![
                "ds plugin list",
                "ds plugin install ds-plugin-docker",
                "ds plugin search kubernetes",
                "ds plugin create my-plugin",
            ],
            flags: vec![],
            subcommands: vec![
                ("list", "List installed plugins"),
                ("install <name>", "Install a plugin"),
                ("remove <name>", "Remove a plugin"),
                ("update [name]", "Update plugins"),
                ("create <name>", "Scaffold a new plugin project"),
                ("search <query>", "Search for plugins"),
                ("enable <name>", "Enable a plugin"),
                ("disable <name>", "Disable a plugin"),
                ("info <name>", "Show plugin details"),
            ],
            note: None,
        }),
        "config" => Some(CommandHelp {
            description: "Manage DeftShell configuration.",
            usage: "ds config [SUBCOMMAND]",
            examples: vec![
                "ds config",
                "ds config get ai.provider",
                "ds config set ai.provider openai",
                "ds config path",
                "ds config validate",
                "ds config ui",
            ],
            flags: vec![],
            subcommands: vec![
                ("get <key>", "Get a config value"),
                ("set <key> <value>", "Set a config value"),
                ("reset", "Reset to defaults"),
                ("validate", "Validate config"),
                ("path", "Show config file path"),
                ("export", "Export config as JSON"),
                ("import <file>", "Import config from file"),
                ("ui", "Open web config UI"),
            ],
            note: None,
        }),
        "auth" => Some(CommandHelp {
            description: "Manage AI provider authentication.",
            usage: "ds auth <SUBCOMMAND>",
            examples: vec![
                "ds auth status",
                "ds auth openai",
                "ds auth anthropic",
                "ds auth revoke openai",
            ],
            flags: vec![],
            subcommands: vec![
                ("status", "Show all provider auth status"),
                ("revoke <provider>", "Revoke stored credentials"),
                ("<provider>", "Authenticate with a specific provider"),
            ],
            note: None,
        }),
        "privacy" => Some(CommandHelp {
            description: "Control DeftShell privacy settings.",
            usage: "ds privacy <on|off>",
            examples: vec![
                "ds privacy on",
                "ds privacy off",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("When privacy is on, no data is sent to external services."),
        }),
        "stats" => Some(CommandHelp {
            description: "View analytics and usage statistics.",
            usage: "ds stats [period] [OPTIONS]",
            examples: vec![
                "ds stats",
                "ds stats today",
                "ds stats week",
                "ds stats commands",
                "ds stats --format json",
            ],
            flags: vec![
                ("--format <fmt>", "Export format: json or csv"),
            ],
            subcommands: vec![],
            note: Some("Periods: today, week, project, commands, errors, ai."),
        }),
        "usage" => Some(CommandHelp {
            description: "View AI usage and cost tracking information.",
            usage: "ds usage",
            examples: vec!["ds usage"],
            flags: vec![],
            subcommands: vec![],
            note: None,
        }),
        "doctor" => Some(CommandHelp {
            description: "Diagnose common issues with your DeftShell installation.",
            usage: "ds doctor [OPTIONS]",
            examples: vec![
                "ds doctor",
                "ds doctor --verbose",
            ],
            flags: vec![
                ("--verbose", "Show verbose diagnostic information"),
            ],
            subcommands: vec![],
            note: None,
        }),
        "version" => Some(CommandHelp {
            description: "Display DeftShell version, OS, architecture, and shell information.",
            usage: "ds version",
            examples: vec!["ds version"],
            flags: vec![],
            subcommands: vec![],
            note: None,
        }),
        "init" => Some(CommandHelp {
            description: "Generate the shell initialization script that integrates DeftShell into your shell.",
            usage: "ds init <shell>",
            examples: vec![
                "eval \"$(ds init zsh)\"",
                "eval \"$(ds init bash)\"",
                "ds init fish | source",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("Add the eval line to your shell rc file (.zshrc, .bashrc, config.fish)."),
        }),
        "completions" => Some(CommandHelp {
            description: "Generate shell tab-completion scripts for DeftShell commands.",
            usage: "ds completions <shell>",
            examples: vec![
                "ds completions zsh > ~/.deftshell/_ds",
                "ds completions bash > ~/.deftshell/ds.bash",
                "ds completions fish > ~/.config/fish/completions/ds.fish",
            ],
            flags: vec![],
            subcommands: vec![],
            note: Some("Supported shells: zsh, bash, fish, elvish, powershell."),
        }),
        "update" => Some(CommandHelp {
            description: "Check for and install DeftShell updates.",
            usage: "ds update [OPTIONS]",
            examples: vec![
                "ds update",
                "ds update --check",
                "ds update --channel beta",
            ],
            flags: vec![
                ("--channel <ch>", "Update channel: stable, beta, nightly"),
                ("--check", "Check for updates without installing"),
            ],
            subcommands: vec![],
            note: None,
        }),
        _ => None,
    }
}

/// Print a category header.
fn print_category(name: &str) {
    println!("  {}", format!("{}:", name).bold().underline());
}

/// Print a command entry with name and description.
fn print_command(name: &str, description: &str) {
    println!("    {:<30} {}", name.bold(), description.dimmed());
}

/// Print a global option entry.
fn print_option(flag: &str, description: &str) {
    println!("    {:<30} {}", flag.bold(), description.dimmed());
}

/// `ds prompt-segment` - render prompt for shell hooks
pub fn run_prompt_segment(
    shell: &str,
    exit_code: i32,
    duration: u64,
    right: bool,
    config: &DeftShellConfig,
) -> Result<()> {
    let shell_type: ds_core::config::ShellType = shell.parse()?;
    let cwd = std::env::current_dir()?;

    let renderer = ds_core::shell::prompt::PromptRenderer::new(config.prompt.clone());
    let data = renderer.collect_data(&cwd, exit_code, duration, None);

    if right {
        print!("{}", renderer.render_right(&data, shell_type));
    } else {
        print!("{}", renderer.render_left(&data, shell_type));
    }
    Ok(())
}

/// `ds safety-check` - check command safety
pub fn run_safety_check(command: &str, config: &DeftShellConfig) -> Result<()> {
    let interceptor = ds_core::safety::CommandInterceptor::new(&config.safety)?;
    let context = ds_core::safety::InterceptionContext::default();

    if let Some(alert) = interceptor.check(command, &context) {
        let level_icon = match alert.level {
            ds_core::safety::RiskLevel::Critical => "CRITICAL".red().bold(),
            ds_core::safety::RiskLevel::High => "HIGH".red(),
            ds_core::safety::RiskLevel::Medium => "MEDIUM".yellow(),
            ds_core::safety::RiskLevel::Low => "LOW".blue(),
        };

        eprintln!();
        eprintln!(
            "{}",
            "  CAUTION: Destructive Command Detected".yellow().bold()
        );
        eprintln!("{}", "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
        eprintln!();
        eprintln!("  Command:  {}", command.bold());
        eprintln!("  Risk:     {}", level_icon);
        eprintln!("  Reason:   {}", alert.reason);
        if let Some(ref suggestion) = alert.suggestion {
            eprintln!();
            eprintln!("  Suggestion: {}", suggestion.green());
        }
        eprintln!();

        std::process::exit(1);
    }

    Ok(())
}

/// `ds track-command` - track command in history and show suggestions
pub fn run_track_command(command: &str, exit_code: i32, duration: u64, dir: &str) -> Result<()> {
    let db = ds_core::storage::Database::open(&ds_core::config::ConfigLoader::db_path())?;
    let tracker = ds_core::intelligence::CommandTracker::new(&db);
    tracker.track(command, dir, exit_code, duration)?;

    // Run suggestion checks (best-effort — never fail the command)
    let engine = ds_core::intelligence::SuggestionEngine::new(&db);

    // 1. Typo detection (only on failed commands)
    if exit_code == 127 {
        // 127 = command not found
        if let Some(suggestion) = engine.check_typo(command) {
            eprintln!("  {} {}", "ds>".cyan().bold(), suggestion.message);
        }
    }

    // 2. Alias suggestion (periodic check — only after every 50th command to avoid spam)
    let stats = db.get_command_stats(dir).ok();
    let should_check_alias = stats
        .as_ref()
        .is_some_and(|s| s.total_commands % 50 == 0 && s.total_commands > 0);
    if should_check_alias {
        if let Ok(Some(suggestion)) = engine.check_alias_suggestion(dir) {
            eprintln!("  {} {}", "ds>".cyan().bold(), suggestion.message);
            if let Some(action) = &suggestion.action {
                eprintln!("       Run: {}", action.cyan());
            }
        }
    }

    // 3. Sequence detection
    if let Ok(Some(suggestion)) = engine.check_sequence(command, dir) {
        eprintln!("  {} {}", "ds>".cyan().bold(), suggestion.message);
    }

    Ok(())
}
