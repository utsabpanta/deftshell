use anyhow::Result;
use colored::Colorize;
use dialoguer::{Input, Password};

use ds_core::config::DeftShellConfig;
use ds_core::storage::KeychainStore;

use crate::AuthAction;

/// Known AI providers and their credential key names.
const PROVIDERS: &[(&str, &str)] = &[
    ("anthropic", "anthropic_api_key"),
    ("openai", "openai_api_key"),
    ("gemini", "gemini_api_key"),
    ("ollama", "ollama"),
    ("copilot", "copilot_token"),
    ("bedrock", "bedrock_profile"),
];

/// Entry point for `ds auth <action>`.
pub async fn run(action: AuthAction, config: &DeftShellConfig) -> Result<()> {
    match action {
        AuthAction::Status => show_status(config),
        AuthAction::Revoke { provider } => revoke_credentials(&provider),
        AuthAction::Provider(args) => {
            if args.is_empty() {
                anyhow::bail!("Provider name required. Usage: ds auth <provider>");
            }
            authenticate_provider(&args[0], config).await
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-commands
// ---------------------------------------------------------------------------

fn show_status(config: &DeftShellConfig) -> Result<()> {
    let keychain = KeychainStore::new();

    println!(
        "\n{}\n",
        "AI Provider Authentication Status".bold().underline()
    );

    let default_provider = &config.ai.default_provider;

    for &(name, key) in PROVIDERS {
        let has_cred = keychain.get_secret("auth", key).is_some();
        let is_configured = config.ai.providers.get(name).is_some_and(|p| p.enabled);

        // Special check for copilot: also detect gh CLI and hosts.json tokens.
        let has_copilot_token = name == "copilot" && !has_cred && detect_copilot_token();

        let status = if has_cred || has_copilot_token {
            if has_copilot_token {
                "authenticated (via gh CLI)".green().to_string()
            } else {
                "authenticated".green().to_string()
            }
        } else if is_configured {
            "configured (no credentials)".yellow().to_string()
        } else {
            "not configured".yellow().to_string()
        };

        let default_badge = if name == default_provider {
            " (default)".cyan().to_string()
        } else {
            String::new()
        };

        println!("  {}{}  {}", name.bold(), default_badge, status);
    }

    println!(
        "\nAuthenticate a provider with: {}",
        "ds auth <provider>".cyan()
    );
    Ok(())
}

fn revoke_credentials(provider: &str) -> Result<()> {
    let keychain = KeychainStore::new();

    let key = match provider.to_lowercase().as_str() {
        "anthropic" => "anthropic_api_key",
        "openai" => "openai_api_key",
        "gemini" => "gemini_api_key",
        "ollama" => "ollama",
        "copilot" => "copilot_token",
        "bedrock" => "bedrock_profile",
        _ => {
            anyhow::bail!(
                "Unknown provider '{}'. Supported: anthropic, openai, gemini, ollama, copilot, bedrock",
                provider
            );
        }
    };

    keychain.delete_secret("auth", key)?;

    println!(
        "{} Revoked credentials for '{}'.",
        "OK".green().bold(),
        provider.cyan()
    );
    Ok(())
}

async fn authenticate_provider(provider: &str, config: &DeftShellConfig) -> Result<()> {
    match provider.to_lowercase().as_str() {
        "anthropic" => auth_api_key_provider(
            "anthropic",
            "anthropic_api_key",
            "ANTHROPIC_API_KEY",
            "sk-ant-",
        ),
        "openai" => auth_api_key_provider("openai", "openai_api_key", "OPENAI_API_KEY", "sk-"),
        "gemini" => auth_api_key_provider("gemini", "gemini_api_key", "GEMINI_API_KEY", ""),
        "ollama" => auth_ollama(config).await,
        "copilot" => auth_copilot(),
        "bedrock" => auth_bedrock(),
        _ => {
            anyhow::bail!(
                "Unknown provider '{}'. Supported: anthropic, openai, gemini, ollama, copilot, bedrock",
                provider
            );
        }
    }
}

fn auth_api_key_provider(
    name: &str,
    keychain_key: &str,
    env_var: &str,
    expected_prefix: &str,
) -> Result<()> {
    let keychain = KeychainStore::new();

    // Check if already authenticated
    if let Some(_existing) = keychain.get_secret("auth", keychain_key) {
        println!("  {} is already authenticated.", name.cyan().bold());
        let overwrite = dialoguer::Confirm::new()
            .with_prompt("Replace existing API key?")
            .default(false)
            .interact()?;
        if !overwrite {
            return Ok(());
        }
    }

    println!("\n  {} Authentication\n", name.cyan().bold());

    // Check environment variable first
    if let Ok(env_key) = std::env::var(env_var) {
        println!("  Found {} in environment.", env_var.yellow());
        let use_env = dialoguer::Confirm::new()
            .with_prompt("Store this key in credential store?")
            .default(true)
            .interact()?;
        if use_env {
            keychain.store_secret("auth", keychain_key, &env_key)?;
            println!(
                "  {} API key stored in credential store.",
                "OK".green().bold()
            );
            set_default_provider(name);
            return Ok(());
        }
    }

    let api_key: String = Password::new()
        .with_prompt(format!("  Enter your {} API key", name))
        .interact()?;

    if api_key.trim().is_empty() {
        anyhow::bail!("API key cannot be empty.");
    }

    if !expected_prefix.is_empty() && !api_key.starts_with(expected_prefix) {
        println!(
            "  {} API key does not start with expected prefix '{}'.",
            "WARNING".yellow().bold(),
            expected_prefix
        );
    }

    keychain.store_secret("auth", keychain_key, api_key.trim())?;

    println!(
        "\n  {} {} API key stored in credential store.",
        "OK".green().bold(),
        name.cyan()
    );

    // Auto-set as default provider.
    set_default_provider(name);

    println!("\n  Try it: {}", "ds ask \"hello\"".cyan());
    Ok(())
}

async fn auth_ollama(config: &DeftShellConfig) -> Result<()> {
    let host = config
        .ai
        .providers
        .get("ollama")
        .and_then(|p| p.host.as_deref())
        .unwrap_or("http://localhost:11434");

    println!("\n  {} Authentication\n", "Ollama".cyan().bold());
    println!("  Testing connection to {}...", host.yellow());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    match client.get(host).send().await {
        Ok(response) if response.status().is_success() => {
            println!("  {} Ollama is running at {}", "OK".green().bold(), host);

            // Try to list available models
            let models_url = format!("{}/api/tags", host);
            if let Ok(resp) = client.get(&models_url).send().await {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    if let Some(models) = body["models"].as_array() {
                        if !models.is_empty() {
                            println!("\n  Available models:");
                            for model in models {
                                if let Some(name) = model["name"].as_str() {
                                    let size = model["size"]
                                        .as_u64()
                                        .map(|s| format!(" ({:.1} GB)", s as f64 / 1_073_741_824.0))
                                        .unwrap_or_default();
                                    println!("    - {}{}", name.cyan(), size.dimmed());
                                }
                            }
                        }
                    }
                }
            }

            // Mark as available in keychain (just a marker)
            let keychain = KeychainStore::new();
            keychain.store_secret("auth", "ollama", host)?;
        }
        Ok(response) => {
            println!(
                "  {} Ollama returned status {} at {}",
                "WARNING".yellow().bold(),
                response.status(),
                host
            );
        }
        Err(e) => {
            println!(
                "  {} Cannot connect to Ollama at {}: {}",
                "ERROR".red().bold(),
                host,
                e
            );
            println!("\n  Make sure Ollama is running:");
            println!("    Install: {}", "https://ollama.com".cyan());
            println!("    Start:   {}", "ollama serve".cyan());
        }
    }
    Ok(())
}

fn auth_copilot() -> Result<()> {
    println!("\n  {} Authentication\n", "GitHub Copilot".cyan().bold());

    // Check if gh is available and authenticated
    let gh_status = std::process::Command::new("gh")
        .args(["auth", "status"])
        .output();

    match gh_status {
        Ok(output) if output.status.success() => {
            println!("  {} GitHub CLI is authenticated.", "OK".green().bold());

            // Set as default provider
            set_default_provider("copilot");

            println!("\n  You're all set! Try: {}", "ds ask \"hello\"".cyan());
        }
        Ok(_) => {
            println!(
                "  {} GitHub CLI is not authenticated.\n",
                "WARNING".yellow().bold()
            );
            println!("  Run these commands:");
            println!("    1. {}", "gh auth login".cyan());
            println!("    2. {}", "ds auth copilot".cyan());
        }
        Err(_) => {
            println!("  {} GitHub CLI (gh) not found.\n", "ERROR".red().bold());
            println!("  Install it from: {}", "https://cli.github.com".cyan());
            println!("  Then run:");
            println!("    gh auth login");
            println!("    ds auth copilot");
        }
    }
    Ok(())
}

fn auth_bedrock() -> Result<()> {
    let keychain = KeychainStore::new();

    println!("\n  {} Authentication\n", "AWS Bedrock".cyan().bold());
    println!("  AWS Bedrock uses AWS credentials for authentication.");
    println!();

    let profile: String = Input::new()
        .with_prompt("  AWS profile name")
        .default("default".to_string())
        .interact_text()?;

    // Verify the profile exists
    match std::process::Command::new("aws")
        .args(["sts", "get-caller-identity", "--profile", &profile])
        .output()
    {
        Ok(output) if output.status.success() => {
            let body = String::from_utf8_lossy(&output.stdout);
            println!(
                "\n  {} AWS profile '{}' is valid.",
                "OK".green().bold(),
                profile.cyan()
            );
            if let Ok(identity) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(arn) = identity["Arn"].as_str() {
                    println!("  Identity: {}", arn.dimmed());
                }
            }

            keychain.store_secret("auth", "bedrock_profile", &profile)?;
            println!(
                "  {} AWS Bedrock profile stored in credential store.",
                "OK".green().bold()
            );
        }
        Ok(_) => {
            println!(
                "\n  {} AWS profile '{}' authentication failed.",
                "ERROR".red().bold(),
                profile
            );
            println!("  Configure AWS credentials:");
            println!("    {}", "aws configure --profile <name>".cyan());
        }
        Err(_) => {
            println!(
                "\n  {} AWS CLI not found. Install it from {}",
                "ERROR".red().bold(),
                "https://aws.amazon.com/cli/".cyan()
            );
            // Store profile anyway, user might configure AWS later
            keychain.store_secret("auth", "bedrock_profile", &profile)?;
            println!("  Stored profile name '{}' for later use.", profile.cyan());
        }
    }
    Ok(())
}

/// Auto-set a provider as the default after authentication.
fn set_default_provider(name: &str) {
    let config = ds_core::config::ConfigLoader::load(None);
    let mut config = config.unwrap_or_default();
    config.ai.default_provider = name.to_string();
    if let Err(e) = ds_core::config::ConfigLoader::save_user_config(&config) {
        println!(
            "  {}: could not save default provider: {}",
            "WARNING".yellow().bold(),
            e
        );
        println!(
            "  Run manually: {}",
            format!("ds config set ai.default_provider {}", name).cyan()
        );
    } else {
        println!(
            "  {} Default AI provider set to {}.",
            "OK".green().bold(),
            name.cyan()
        );
    }
}

/// Detect if a Copilot-compatible token is available via `gh auth token`
/// or `~/.config/github-copilot/hosts.json`.
fn detect_copilot_token() -> bool {
    // Check gh CLI
    if let Ok(output) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
    {
        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout);
            if !token.trim().is_empty() {
                return true;
            }
        }
    }

    // Check hosts.json
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".config/github-copilot/hosts.json");
        if path.exists() {
            return true;
        }
    }

    false
}
