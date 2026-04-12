use anyhow::{bail, Result};
use colored::Colorize;

use ds_core::config::{ConfigLoader, DeftShellConfig};
use ds_core::plugin::{PluginLoader, PluginRegistry};

use crate::PluginAction;

/// Entry point for `ds plugin <action>`.
pub async fn run(action: PluginAction, _config: &DeftShellConfig) -> Result<()> {
    let plugins_dir = ConfigLoader::data_dir().join("plugins");

    match action {
        PluginAction::List => list_plugins(&plugins_dir),
        PluginAction::Install { name } => install_plugin(&name, &plugins_dir),
        PluginAction::Remove { name } => remove_plugin(&name, &plugins_dir),
        PluginAction::Update { name } => update_plugin(name.as_deref(), &plugins_dir),
        PluginAction::Create { name } => create_plugin(&name),
        PluginAction::Search { query } => search_plugins(&query).await,
        PluginAction::Enable { name } => enable_plugin(&name, &plugins_dir),
        PluginAction::Disable { name } => disable_plugin(&name, &plugins_dir),
        PluginAction::Info { name } => show_plugin_info(&name, &plugins_dir),
    }
}

// ---------------------------------------------------------------------------
// Sub-commands
// ---------------------------------------------------------------------------

fn list_plugins(plugins_dir: &std::path::Path) -> Result<()> {
    let loader = PluginLoader::new(plugins_dir.to_path_buf());
    let plugins = loader.load_all()?;

    if plugins.is_empty() {
        println!("{}", "No plugins installed.".dimmed());
        println!("Search for plugins: {}", "ds plugin search <query>".cyan());
        println!("Install a plugin:   {}", "ds plugin install <name>".cyan());
        return Ok(());
    }

    println!("{}\n", "Installed Plugins".bold().underline());

    for plugin in &plugins {
        let status = if plugin.enabled {
            "enabled".green().to_string()
        } else {
            "disabled".dimmed().to_string()
        };

        println!(
            "  {}  v{}  [{}]",
            plugin.manifest.name.cyan().bold(),
            plugin.manifest.version,
            status
        );
        if !plugin.manifest.description.is_empty() {
            println!("    {}", plugin.manifest.description.dimmed());
        }
        println!(
            "    Type: {}  Path: {}",
            plugin.manifest.plugin_type,
            plugin.path.display().to_string().dimmed()
        );
        println!();
    }

    println!("  {} plugin(s) installed", plugins.len().to_string().bold());
    Ok(())
}

fn install_plugin(name: &str, plugins_dir: &std::path::Path) -> Result<()> {
    let loader = PluginLoader::new(plugins_dir.to_path_buf());
    let path = std::path::Path::new(name);

    // If name looks like a local path, install from local directory
    let plugin = if path.exists() && path.is_dir() {
        println!("Installing plugin from local path: {}", name.cyan());
        loader.install_from_local(path)?
    } else {
        println!("Installing plugin '{}' from npm...", name.cyan());
        loader.install_from_npm(name)?
    };

    println!(
        "\n{} Installed '{}' v{} ({})",
        "OK".green().bold(),
        plugin.manifest.name.cyan(),
        plugin.manifest.version,
        plugin.manifest.plugin_type
    );
    if !plugin.manifest.description.is_empty() {
        println!("  {}", plugin.manifest.description);
    }
    Ok(())
}

fn remove_plugin(name: &str, plugins_dir: &std::path::Path) -> Result<()> {
    let loader = PluginLoader::new(plugins_dir.to_path_buf());
    let plugin_dir = plugins_dir.join(name);

    if !plugin_dir.exists() {
        bail!(
            "Plugin '{}' is not installed. Run {} to see installed plugins.",
            name,
            "ds plugin list".cyan()
        );
    }

    let confirmed = dialoguer::Confirm::new()
        .with_prompt(format!("Remove plugin '{}'?", name))
        .default(false)
        .interact()?;

    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    loader.remove(name)?;
    println!("{} Removed plugin '{}'.", "OK".green().bold(), name.cyan());
    Ok(())
}

fn update_plugin(name: Option<&str>, plugins_dir: &std::path::Path) -> Result<()> {
    let loader = PluginLoader::new(plugins_dir.to_path_buf());

    match name {
        Some(plugin_name) => {
            let plugin_dir = plugins_dir.join(plugin_name);
            if !plugin_dir.exists() {
                bail!("Plugin '{}' is not installed.", plugin_name);
            }
            println!("Updating plugin '{}'...", plugin_name.cyan());
            // Re-install from npm to update
            loader.remove(plugin_name)?;
            let updated = loader.install_from_npm(plugin_name)?;
            println!(
                "{} Updated '{}' to v{}",
                "OK".green().bold(),
                updated.manifest.name.cyan(),
                updated.manifest.version
            );
        }
        None => {
            println!("Updating all plugins...\n");
            let plugins = loader.load_all()?;
            if plugins.is_empty() {
                println!("{}", "No plugins to update.".dimmed());
                return Ok(());
            }
            let mut updated_count = 0;
            for plugin in &plugins {
                let name = &plugin.manifest.name;
                print!("  Updating {}...", name.cyan());
                match PluginLoader::new(plugins_dir.to_path_buf()).install_from_npm(name) {
                    Ok(p) => {
                        println!(" {} v{}", "OK".green(), p.manifest.version);
                        updated_count += 1;
                    }
                    Err(e) => {
                        println!(" {} {}", "FAIL".red(), e);
                    }
                }
            }
            println!(
                "\n{} Updated {} plugin(s).",
                "OK".green().bold(),
                updated_count
            );
        }
    }
    Ok(())
}

fn create_plugin(name: &str) -> Result<()> {
    let dest = std::env::current_dir()?.join(name);

    if dest.exists() {
        bail!("Directory '{}' already exists.", dest.display());
    }

    println!(
        "Scaffolding new plugin '{}' at {}",
        name.cyan(),
        dest.display()
    );

    PluginRegistry::scaffold(name, &dest)?;

    println!(
        "\n{} Plugin '{}' created.\n",
        "OK".green().bold(),
        name.cyan()
    );
    println!("  Next steps:");
    println!("    1. cd {}", name);
    println!("    2. Edit {} to add your commands", "index.js".cyan());
    println!(
        "    3. Test locally with: {}",
        format!("ds plugin install ./{}", name).cyan()
    );
    println!("    4. Publish to npm:    {}", "npm publish".cyan());
    Ok(())
}

async fn search_plugins(query: &str) -> Result<()> {
    println!(
        "Searching npm for DeftShell plugins matching '{}'...\n",
        query.cyan()
    );

    let registry = PluginRegistry::new();
    let results = registry.search(query).await?;

    if results.is_empty() {
        println!("{}", "No plugins found.".dimmed());
        return Ok(());
    }

    for result in &results {
        let author = result
            .author
            .as_ref()
            .and_then(|a| a.name.as_deref())
            .unwrap_or("unknown");

        println!(
            "  {} v{}  by {}",
            result.name.cyan().bold(),
            result.version,
            author.dimmed()
        );
        if let Some(ref desc) = result.description {
            println!("    {}", desc);
        }
        if !result.keywords.is_empty() {
            println!("    Keywords: {}", result.keywords.join(", ").dimmed());
        }
        println!();
    }

    println!("Install with: {}", "ds plugin install <name>".cyan());
    Ok(())
}

fn enable_plugin(name: &str, plugins_dir: &std::path::Path) -> Result<()> {
    let plugin_dir = plugins_dir.join(name);
    if !plugin_dir.exists() {
        bail!("Plugin '{}' is not installed.", name);
    }

    let loader = PluginLoader::new(plugins_dir.to_path_buf());
    loader.enable(name)?;

    println!("{} Plugin '{}' enabled.", "OK".green().bold(), name.cyan());
    Ok(())
}

fn disable_plugin(name: &str, plugins_dir: &std::path::Path) -> Result<()> {
    let plugin_dir = plugins_dir.join(name);
    if !plugin_dir.exists() {
        bail!("Plugin '{}' is not installed.", name);
    }

    let loader = PluginLoader::new(plugins_dir.to_path_buf());
    loader.disable(name)?;

    println!("{} Plugin '{}' disabled.", "OK".green().bold(), name.cyan());
    Ok(())
}

fn show_plugin_info(name: &str, plugins_dir: &std::path::Path) -> Result<()> {
    let loader = PluginLoader::new(plugins_dir.to_path_buf());
    let plugin_dir = plugins_dir.join(name);

    if !plugin_dir.exists() {
        bail!(
            "Plugin '{}' is not installed. Run {} to see installed plugins.",
            name,
            "ds plugin list".cyan()
        );
    }

    let plugin = loader.load_plugin(&plugin_dir)?;

    println!();
    println!(
        "  {} {}",
        "Plugin:".bold(),
        plugin.manifest.name.cyan().bold()
    );
    println!("  {} {}", "Version:".bold(), plugin.manifest.version);
    println!("  {} {}", "Type:".bold(), plugin.manifest.plugin_type);
    println!(
        "  {} {}",
        "Description:".bold(),
        plugin.manifest.description
    );
    println!("  {} {}", "Author:".bold(), plugin.manifest.author);

    let status = if plugin.enabled {
        "enabled".green().to_string()
    } else {
        "disabled".dimmed().to_string()
    };
    println!("  {} {}", "Status:".bold(), status);

    if let Some(ref entry) = plugin.manifest.entry_point {
        println!("  {} {}", "Entry point:".bold(), entry);
    }
    if let Some(ref homepage) = plugin.manifest.homepage {
        println!("  {} {}", "Homepage:".bold(), homepage);
    }
    if let Some(ref repo) = plugin.manifest.repository {
        println!("  {} {}", "Repository:".bold(), repo);
    }
    if !plugin.manifest.keywords.is_empty() {
        println!(
            "  {} {}",
            "Keywords:".bold(),
            plugin.manifest.keywords.join(", ")
        );
    }
    println!("  {} {}", "Path:".bold(), plugin.path.display());
    println!();
    Ok(())
}
