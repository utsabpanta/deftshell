use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::collections::HashMap;

use ds_core::config::{ConfigLoader, DeftShellConfig};

use crate::{AliasAction, ConfigAction};

/// Entry point for `ds config [action]`.
pub fn run(action: Option<ConfigAction>, config: &DeftShellConfig) -> Result<()> {
    match action {
        Some(ConfigAction::Get { key }) => config_get(&key, config),
        Some(ConfigAction::Set { key, value }) => config_set(&key, &value, config),
        Some(ConfigAction::Reset) => config_reset(),
        Some(ConfigAction::Validate) => config_validate(),
        Some(ConfigAction::Path) => config_path(),
        Some(ConfigAction::Export) => config_export(config),
        Some(ConfigAction::Import { file }) => config_import(&file),
        Some(ConfigAction::Ui) => config_ui(),
        None => config_open_editor(),
    }
}

/// `ds privacy <on|off>` - toggle privacy mode.
pub fn run_privacy(mode: &str, config: &DeftShellConfig) -> Result<()> {
    let mut config = config.clone();

    match mode.to_lowercase().as_str() {
        "on" | "true" | "1" | "enable" => {
            config.ai.privacy_mode = true;
            ConfigLoader::save_user_config(&config)?;
            println!(
                "{} Privacy mode {}.",
                "OK".green().bold(),
                "enabled".green().bold()
            );
            println!(
                "  AI requests will be routed through: {}",
                config
                    .ai
                    .privacy_mode_provider
                    .as_deref()
                    .unwrap_or("ollama")
                    .cyan()
            );
            println!("  No data will be sent to cloud providers.");
        }
        "off" | "false" | "0" | "disable" => {
            config.ai.privacy_mode = false;
            ConfigLoader::save_user_config(&config)?;
            println!(
                "{} Privacy mode {}.",
                "OK".green().bold(),
                "disabled".yellow().bold()
            );
            println!(
                "  AI requests will use default provider: {}",
                config.ai.default_provider.cyan()
            );
        }
        _ => {
            bail!(
                "Invalid privacy mode '{}'. Use {} or {}.",
                mode,
                "on".green(),
                "off".yellow()
            );
        }
    }
    Ok(())
}

/// `ds alias [action]` - manage aliases.
pub fn run_alias(action: Option<AliasAction>, export: bool, shell: Option<&str>) -> Result<()> {
    let aliases_path = ConfigLoader::data_dir().join("aliases.toml");

    if export {
        return export_aliases(&aliases_path, shell);
    }

    match action {
        Some(AliasAction::Add { spec }) => add_alias(&spec, &aliases_path),
        Some(AliasAction::Remove { name }) => remove_alias(&name, &aliases_path),
        Some(AliasAction::List) | None => list_aliases(&aliases_path),
    }
}

// ---------------------------------------------------------------------------
// Config sub-commands
// ---------------------------------------------------------------------------

fn config_get(key: &str, config: &DeftShellConfig) -> Result<()> {
    // Serialize config to a serde_json::Value for dot-notation access
    let json = serde_json::to_value(config)?;
    let value = resolve_dot_key(&json, key);

    match value {
        Some(v) => {
            let display = match v {
                serde_json::Value::String(s) => s.to_string(),
                other => serde_json::to_string_pretty(&other)?,
            };
            println!("{}", display);
        }
        None => {
            bail!("Config key '{}' not found.", key);
        }
    }
    Ok(())
}

fn config_set(key: &str, value: &str, config: &DeftShellConfig) -> Result<()> {
    // Serialize current config to JSON for dot-notation manipulation
    let json_str = serde_json::to_string(config)?;
    let mut json: serde_json::Value = serde_json::from_str(&json_str)?;

    set_dot_key(&mut json, key, value)?;

    // Deserialize back to config struct to validate
    let updated_config: DeftShellConfig =
        serde_json::from_value(json).with_context(|| format!("Invalid value for key '{}'", key))?;

    ConfigLoader::save_user_config(&updated_config)?;

    println!(
        "{} Set {} = {}",
        "OK".green().bold(),
        key.cyan(),
        value.yellow()
    );
    Ok(())
}

fn config_reset() -> Result<()> {
    let confirmed = dialoguer::Confirm::new()
        .with_prompt("Reset configuration to defaults? This cannot be undone.")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let default_config = DeftShellConfig::default();
    ConfigLoader::save_user_config(&default_config)?;

    println!("{} Configuration reset to defaults.", "OK".green().bold());
    Ok(())
}

fn config_validate() -> Result<()> {
    let config_path = ConfigLoader::user_config_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

    if !config_path.exists() {
        println!(
            "{} No user config file found at {}. Using defaults.",
            "INFO".blue().bold(),
            config_path.display()
        );
        return Ok(());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    match toml::from_str::<DeftShellConfig>(&contents) {
        Ok(_) => {
            println!(
                "{} Configuration is valid. ({})",
                "OK".green().bold(),
                config_path.display().to_string().dimmed()
            );
        }
        Err(e) => {
            println!(
                "{} Configuration has errors:\n  {}",
                "ERROR".red().bold(),
                e
            );
        }
    }
    Ok(())
}

fn config_path() -> Result<()> {
    match ConfigLoader::user_config_path() {
        Some(path) => {
            println!("{}", path.display());
        }
        None => {
            bail!("Could not determine config path (HOME not set).");
        }
    }
    Ok(())
}

fn config_export(config: &DeftShellConfig) -> Result<()> {
    let json = serde_json::to_string_pretty(config)?;
    println!("{}", json);
    Ok(())
}

fn config_import(file: &str) -> Result<()> {
    let path = std::path::Path::new(file);
    if !path.exists() {
        bail!("File not found: {}", file);
    }

    let contents =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", file))?;

    // Try JSON first, then TOML
    let config: DeftShellConfig = if file.ends_with(".json") {
        serde_json::from_str(&contents).with_context(|| "Failed to parse JSON config")?
    } else {
        toml::from_str(&contents).with_context(|| "Failed to parse TOML config")?
    };

    ConfigLoader::save_user_config(&config)?;

    println!(
        "{} Imported configuration from {}",
        "OK".green().bold(),
        file.cyan()
    );
    Ok(())
}

fn config_ui() -> Result<()> {
    println!(
        "{} Web-based configuration UI is not yet available.",
        "INFO".blue().bold()
    );
    println!(
        "Edit your config with: {} or {}",
        "ds config".cyan(),
        "ds config set <key> <value>".cyan()
    );
    Ok(())
}

fn config_open_editor() -> Result<()> {
    let config_path = ConfigLoader::user_config_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

    // Create default config file if it doesn't exist
    if !config_path.exists() {
        let default_config = DeftShellConfig::default();
        ConfigLoader::save_user_config(&default_config)?;
        println!(
            "Created default config at {}",
            config_path.display().to_string().dimmed()
        );
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&config_path)
        .status()
        .with_context(|| format!("Failed to open editor '{}'", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    // Validate after editing
    let contents = std::fs::read_to_string(&config_path)?;
    match toml::from_str::<DeftShellConfig>(&contents) {
        Ok(_) => println!("{} Configuration saved and validated.", "OK".green().bold()),
        Err(e) => println!(
            "{} Configuration saved but has parse errors: {}",
            "WARNING".yellow().bold(),
            e
        ),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Alias sub-commands
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct AliasFile {
    #[serde(default)]
    aliases: HashMap<String, String>,
}

fn load_aliases(path: &std::path::Path) -> Result<AliasFile> {
    if !path.exists() {
        return Ok(AliasFile::default());
    }
    let contents = std::fs::read_to_string(path)?;
    let file: AliasFile = toml::from_str(&contents)?;
    Ok(file)
}

fn save_aliases(path: &std::path::Path, file: &AliasFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(file)?;
    std::fs::write(path, contents)?;
    Ok(())
}

fn add_alias(spec: &str, aliases_path: &std::path::Path) -> Result<()> {
    let pos = spec.find('=').ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid format. Expected alias=command, e.g.: {}",
            "gp='git push'".cyan()
        )
    })?;

    let name = spec[..pos].trim().to_string();
    let command = spec[pos + 1..]
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .to_string();

    if name.is_empty() || command.is_empty() {
        bail!("Alias name and command must not be empty.");
    }

    let mut file = load_aliases(aliases_path)?;
    file.aliases.insert(name.clone(), command.clone());
    save_aliases(aliases_path, &file)?;

    println!(
        "{} Added alias: {} = '{}'",
        "OK".green().bold(),
        name.cyan(),
        command
    );
    println!(
        "  Apply with: {} or restart your shell",
        "source <(ds alias --export)".dimmed()
    );
    Ok(())
}

fn remove_alias(name: &str, aliases_path: &std::path::Path) -> Result<()> {
    let mut file = load_aliases(aliases_path)?;

    if file.aliases.remove(name).is_none() {
        bail!("Alias '{}' not found.", name);
    }

    save_aliases(aliases_path, &file)?;

    println!("{} Removed alias '{}'.", "OK".green().bold(), name.cyan());
    Ok(())
}

fn list_aliases(aliases_path: &std::path::Path) -> Result<()> {
    let file = load_aliases(aliases_path)?;

    if file.aliases.is_empty() {
        println!("{}", "No aliases configured.".dimmed());
        println!("Add one with: {}", "ds alias add gp='git push'".cyan());
        return Ok(());
    }

    println!("{}\n", "Aliases".bold().underline());

    let mut sorted: Vec<_> = file.aliases.iter().collect();
    sorted.sort_by_key(|(name, _)| name.to_string());

    for (name, command) in &sorted {
        println!("  {} = '{}'", name.cyan().bold(), command);
    }

    println!(
        "\n  {} alias(es) configured",
        file.aliases.len().to_string().bold()
    );
    Ok(())
}

fn export_aliases(aliases_path: &std::path::Path, shell: Option<&str>) -> Result<()> {
    let file = load_aliases(aliases_path)?;
    let shell_type = shell.unwrap_or("zsh");

    for (name, command) in &file.aliases {
        // Validate alias name: only allow alphanumeric, hyphens, underscores.
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            eprintln!(
                "WARNING: skipping alias '{}' — name contains invalid characters",
                name
            );
            continue;
        }
        // Escape single quotes in the command value to prevent shell injection.
        let escaped_command = command.replace('\'', "'\\''");
        match shell_type {
            "fish" => println!("alias {} '{}'", name, escaped_command),
            _ => println!("alias {}='{}'", name, escaped_command),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Dot-notation helpers
// ---------------------------------------------------------------------------

fn resolve_dot_key<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

fn set_dot_key(value: &mut serde_json::Value, key: &str, new_value: &str) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last key -- set the value
            if let serde_json::Value::Object(ref mut map) = current {
                // Try to preserve the original type
                if let Some(existing) = map.get(*part) {
                    let typed_value = match existing {
                        serde_json::Value::Bool(_) => match new_value {
                            "true" | "1" | "yes" => serde_json::Value::Bool(true),
                            "false" | "0" | "no" => serde_json::Value::Bool(false),
                            _ => bail!("Expected boolean value for '{}'", key),
                        },
                        serde_json::Value::Number(_) => {
                            if let Ok(n) = new_value.parse::<i64>() {
                                serde_json::Value::Number(n.into())
                            } else if let Ok(n) = new_value.parse::<f64>() {
                                serde_json::json!(n)
                            } else {
                                bail!("Expected numeric value for '{}'", key);
                            }
                        }
                        _ => serde_json::Value::String(new_value.to_string()),
                    };
                    map.insert(part.to_string(), typed_value);
                } else {
                    map.insert(
                        part.to_string(),
                        serde_json::Value::String(new_value.to_string()),
                    );
                }
            } else {
                bail!("Cannot set key '{}': parent is not an object", key);
            }
        } else {
            // Navigate deeper
            if let serde_json::Value::Object(ref mut map) = current {
                current = map
                    .entry(part.to_string())
                    .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            } else {
                bail!("Cannot navigate key '{}': '{}' is not an object", key, part);
            }
        }
    }

    Ok(())
}
