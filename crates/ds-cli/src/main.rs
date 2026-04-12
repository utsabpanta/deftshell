use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod commands;
mod tui;

#[derive(Parser)]
#[command(
    name = "ds",
    version,
    about = "DeftShell — The AI-Powered Context-Aware Terminal for Developers",
    long_about = "DeftShell makes your terminal intelligent, context-aware, and AI-powered.\nAuto-detects project context, integrates with multiple AI providers,\nintercepts dangerous commands, and provides a plugin ecosystem."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Override AI provider for this command
    #[arg(long, global = true)]
    provider: Option<String>,

    /// Auto-confirm all prompts
    #[arg(short = 'y', long, global = true)]
    yes: bool,

    /// Show what would be done without executing
    #[arg(long, global = true)]
    dry_run: bool,

    /// Write output to a file
    #[arg(long, global = true)]
    output: Option<String>,
}

#[derive(Subcommand)]
#[command(disable_help_subcommand = true)]
enum Commands {
    /// Generate shell initialization script
    Init {
        /// Shell type: zsh, bash, or fish
        shell: String,
    },

    /// Ask AI a question with project context
    Ask {
        /// The question to ask
        query: Vec<String>,
    },

    /// AI generates and executes commands
    Do {
        /// Natural language instruction
        instruction: Vec<String>,
    },

    /// Get project-aware how-to instructions
    How {
        /// The question
        question: Vec<String>,
    },

    /// Explain piped command output
    Explain,

    /// Review piped code changes
    Review,

    /// Interactive AI chat mode
    Chat {
        /// Resume last conversation
        #[arg(long)]
        r#continue: bool,
        /// Include file in context
        #[arg(long)]
        context: Option<String>,
    },

    /// AI code generation
    Generate {
        /// Type: component, migration, test, dockerfile, github-action
        r#type: String,
        /// Name of the generated item
        name: Option<String>,
    },

    /// Show detected project context
    Context {
        #[command(subcommand)]
        action: Option<ContextAction>,
        /// Detect context quietly (used by shell hooks)
        #[arg(long)]
        detect: bool,
        /// Suppress output
        #[arg(long)]
        quiet: bool,
    },

    /// List project scripts
    Scripts,

    /// Run a project script
    Run {
        /// Script name
        script: String,
    },

    /// Monorepo workspace commands
    Workspace {
        #[command(subcommand)]
        action: Option<WorkspaceAction>,
    },

    /// Runbook management
    Runbook {
        #[command(subcommand)]
        action: RunbookAction,
    },

    /// Plugin management
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },

    /// AI provider authentication
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Analytics dashboard
    Stats {
        /// Period: today, week, project, commands, errors, ai
        period: Option<String>,
        /// Export format: json or csv
        #[arg(long)]
        format: Option<String>,
    },

    /// Privacy controls
    Privacy {
        /// on or off
        mode: String,
    },

    /// AI usage and cost tracking
    Usage,

    /// Manage context-aware aliases
    Alias {
        #[command(subcommand)]
        action: Option<AliasAction>,
        /// Export aliases for shell sourcing
        #[arg(long)]
        export: bool,
        /// Shell type for export
        #[arg(long)]
        shell: Option<String>,
    },

    /// Environment context display
    Env,

    /// Diagnose issues
    Doctor {
        /// Show verbose diagnostic info
        #[arg(long)]
        verbose: bool,
    },

    /// Self-update
    Update {
        /// Update channel: stable, beta, nightly
        #[arg(long)]
        channel: Option<String>,
        /// Check only, don't install
        #[arg(long)]
        check: bool,
    },

    /// Version info
    Version,

    /// Generate shell completions
    Completions {
        /// Shell type: zsh, bash, fish
        shell: String,
    },

    /// Help for a specific command
    #[command(hide = true)]
    Help {
        /// Command name
        command: Option<String>,
    },

    /// Render prompt segment (used internally by shell hooks)
    #[command(hide = true)]
    PromptSegment {
        #[arg(long)]
        shell: String,
        #[arg(long, default_value = "0")]
        exit_code: i32,
        #[arg(long, default_value = "0")]
        duration: u64,
        #[arg(long)]
        right: bool,
    },

    /// Safety check command (used internally by shell hooks)
    #[command(hide = true)]
    SafetyCheck { command: String },

    /// Track command (used internally by shell hooks)
    #[command(hide = true)]
    TrackCommand {
        #[arg(long)]
        command: String,
        #[arg(long)]
        exit_code: i32,
        #[arg(long)]
        duration: u64,
        #[arg(long)]
        dir: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ContextAction {
    /// Force re-detection
    Refresh,
    /// Export context as JSON
    Export,
    /// Show changes since last detection
    Diff,
}

#[derive(Subcommand)]
pub(crate) enum WorkspaceAction {
    /// List all packages in the monorepo
    List,
}

#[derive(Subcommand)]
pub(crate) enum RunbookAction {
    /// Create a new runbook
    New { name: String },
    /// Edit a runbook
    Edit { name: String },
    /// Delete a runbook
    Delete { name: String },
    /// List all runbooks
    List,
    /// Display runbook steps
    Show { name: String },
    /// Execute a runbook
    Run {
        name: String,
        #[arg(long)]
        from_step: Option<usize>,
        #[arg(long, value_parser = parse_var)]
        var: Vec<(String, String)>,
    },
    /// Start recording commands
    Record { name: Option<String> },
    /// Stop recording
    Stop,
    /// AI-generate a runbook
    Generate { description: Vec<String> },
    /// Search community registry
    Search { query: String },
    /// Install from registry
    Install { spec: String },
    /// Publish to registry
    Publish { name: String },
    /// Show trending runbooks
    Trending,
}

fn parse_var(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid variable format: {s} (expected key=value)"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

#[derive(Subcommand)]
pub(crate) enum PluginAction {
    /// List installed plugins
    List,
    /// Install a plugin
    Install { name: String },
    /// Remove a plugin
    Remove { name: String },
    /// Update plugins
    Update { name: Option<String> },
    /// Scaffold a new plugin project
    Create { name: String },
    /// Search npm for plugins
    Search { query: String },
    /// Enable a plugin
    Enable { name: String },
    /// Disable a plugin
    Disable { name: String },
    /// Show plugin details
    Info { name: String },
}

#[derive(Subcommand)]
pub(crate) enum AuthAction {
    /// Show all provider auth status
    Status,
    /// Revoke stored credentials
    Revoke { provider: String },
    /// Authenticate with a specific provider
    #[command(external_subcommand)]
    Provider(Vec<String>),
}

#[derive(Subcommand)]
pub(crate) enum ConfigAction {
    /// Get a config value
    Get { key: String },
    /// Set a config value
    Set { key: String, value: String },
    /// Reset to defaults
    Reset,
    /// Validate config
    Validate,
    /// Show config file path
    Path,
    /// Export config as JSON
    Export,
    /// Import config from file
    Import { file: String },
    /// Open web config UI
    Ui,
}

#[derive(Subcommand)]
pub(crate) enum AliasAction {
    /// Add an alias
    Add {
        /// alias=command format
        spec: String,
    },
    /// Remove an alias
    Remove { name: String },
    /// List all aliases
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("DS_LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    // Ensure data directory exists
    ds_core::config::ConfigLoader::ensure_data_dir()?;

    // Load configuration
    let cwd = std::env::current_dir()?;
    let config = ds_core::config::ConfigLoader::load(Some(&cwd))?;

    match cli.command {
        Commands::Init { shell } => commands::init::run(&shell),
        Commands::Ask { query } => {
            commands::ask::run(&query.join(" "), &config, cli.provider.as_deref()).await
        }
        Commands::Do { instruction } => {
            commands::do_cmd::run(
                &instruction.join(" "),
                &config,
                cli.yes,
                cli.provider.as_deref(),
            )
            .await
        }
        Commands::How { question } => commands::ask::run_how(&question.join(" "), &config).await,
        Commands::Explain => commands::ask::run_explain(&config).await,
        Commands::Review => commands::ask::run_review(&config).await,
        Commands::Chat {
            r#continue,
            context,
        } => {
            commands::chat::run(
                r#continue,
                context.as_deref(),
                &config,
                cli.provider.as_deref(),
            )
            .await
        }
        Commands::Generate { r#type, name } => {
            commands::ask::run_generate(&r#type, name.as_deref(), &config).await
        }
        Commands::Context {
            action,
            detect,
            quiet,
        } => commands::context::run(action, detect, quiet, &config),
        Commands::Scripts => commands::context::run_scripts(&config),
        Commands::Run { script } => commands::context::run_script(&script, &config),
        Commands::Workspace { action } => commands::context::run_workspace(action),
        Commands::Runbook { action } => commands::runbook::run(action, &config, cli.yes).await,
        Commands::Plugin { action } => commands::plugin::run(action, &config).await,
        Commands::Auth { action } => commands::auth::run(action, &config).await,
        Commands::Config { action } => commands::config::run(action, &config),
        Commands::Stats { period, format } => {
            commands::stats::run(period.as_deref(), format.as_deref(), &config)
        }
        Commands::Privacy { mode } => commands::config::run_privacy(&mode, &config),
        Commands::Usage => commands::stats::run_usage(&config),
        Commands::Alias {
            action,
            export,
            shell,
        } => commands::config::run_alias(action, export, shell.as_deref()),
        Commands::Env => commands::context::run_env(&config),
        Commands::Doctor { verbose } => commands::doctor::run(verbose),
        Commands::Update { channel, check } => {
            commands::init::run_update(channel.as_deref(), check)
        }
        Commands::Version => commands::init::run_version(),
        Commands::Completions { shell } => commands::init::run_completions(&shell),
        Commands::Help { command } => commands::init::run_help(command.as_deref()),
        Commands::PromptSegment {
            shell,
            exit_code,
            duration,
            right,
        } => commands::init::run_prompt_segment(&shell, exit_code, duration, right, &config),
        Commands::SafetyCheck { command } => commands::init::run_safety_check(&command, &config),
        Commands::TrackCommand {
            command,
            exit_code,
            duration,
            dir,
        } => commands::init::run_track_command(&command, exit_code, duration, &dir),
    }
}
