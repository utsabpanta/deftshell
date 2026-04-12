use anyhow::{bail, Context, Result};
use colored::Colorize;
use dialoguer::{Confirm, Input};
use std::collections::HashMap;

use ds_core::config::{ConfigLoader, DeftShellConfig};
use ds_core::runbook::executor::ExecutionOptions;
use ds_core::runbook::{Runbook, RunbookExecutor, RunbookRecorder, RunbookRegistry};

use crate::RunbookAction;

/// Entry point for `ds runbook <action>`.
pub async fn run(
    action: RunbookAction,
    config: &DeftShellConfig,
    auto_confirm: bool,
) -> Result<()> {
    let runbooks_dir = ConfigLoader::data_dir().join("runbooks");

    match action {
        RunbookAction::New { name } => create_new(&name, &runbooks_dir),
        RunbookAction::Edit { name } => edit_runbook(&name, &runbooks_dir),
        RunbookAction::Delete { name } => delete_runbook(&name, &runbooks_dir, auto_confirm),
        RunbookAction::List => list_runbooks(&runbooks_dir),
        RunbookAction::Show { name } => show_runbook(&name, &runbooks_dir),
        RunbookAction::Run {
            name,
            from_step,
            var,
        } => run_runbook(&name, &runbooks_dir, from_step, var, auto_confirm),
        RunbookAction::Record { name } => start_recording(name),
        RunbookAction::Stop => stop_recording(&runbooks_dir),
        RunbookAction::Generate { description } => {
            generate_runbook(&description.join(" "), config, &runbooks_dir).await
        }
        RunbookAction::Search { query } => search_registry(&query).await,
        RunbookAction::Install { spec } => install_from_registry(&spec, &runbooks_dir).await,
        RunbookAction::Publish { name } => publish_runbook(&name),
        RunbookAction::Trending => show_trending().await,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn runbook_path(name: &str, runbooks_dir: &std::path::Path) -> std::path::PathBuf {
    runbooks_dir.join(format!("{}.toml", name))
}

fn load_runbook(name: &str, runbooks_dir: &std::path::Path) -> Result<Runbook> {
    let path = runbook_path(name, runbooks_dir);
    if !path.exists() {
        bail!(
            "Runbook '{}' not found. Run {} to see available runbooks.",
            name,
            "ds runbook list".cyan()
        );
    }
    Runbook::from_file(&path)
}

// ---------------------------------------------------------------------------
// Sub-commands
// ---------------------------------------------------------------------------

fn create_new(name: &str, runbooks_dir: &std::path::Path) -> Result<()> {
    let path = runbook_path(name, runbooks_dir);
    if path.exists() {
        bail!(
            "Runbook '{}' already exists. Use {} to modify it.",
            name,
            "ds runbook edit".cyan()
        );
    }

    let title: String = Input::new()
        .with_prompt("Runbook title")
        .default(name.replace('-', " "))
        .interact_text()?;

    let description: String = Input::new()
        .with_prompt("Description")
        .default(String::new())
        .allow_empty(true)
        .interact_text()?;

    let tags_input: String = Input::new()
        .with_prompt("Tags (comma-separated)")
        .default(String::new())
        .allow_empty(true)
        .interact_text()?;

    let tags: Vec<String> = tags_input
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let author = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    // Collect at least one step
    let mut steps = Vec::new();
    loop {
        let step_title: String = Input::new()
            .with_prompt(format!("Step {} title (empty to finish)", steps.len() + 1))
            .default(String::new())
            .allow_empty(true)
            .interact_text()?;

        if step_title.is_empty() {
            if steps.is_empty() {
                println!(
                    "{}",
                    "At least one step is recommended. You can add steps later by editing the file."
                        .yellow()
                );
            }
            break;
        }

        let command: String = Input::new().with_prompt("  Command").interact_text()?;

        let confirm = Confirm::new()
            .with_prompt("  Require confirmation before running?")
            .default(true)
            .interact()?;

        steps.push(ds_core::runbook::RunbookStep {
            title: step_title,
            command,
            description: String::new(),
            confirm,
            variables: Vec::new(),
            on_failure: ds_core::runbook::parser::OnFailure::Abort,
            fallback_command: None,
            background: false,
        });
    }

    let runbook = Runbook {
        runbook: ds_core::runbook::parser::RunbookMeta {
            name: name.to_string(),
            title,
            description,
            author,
            version: "0.1.0".to_string(),
            tags,
            estimated_time: None,
            requires: Vec::new(),
        },
        steps,
    };

    std::fs::create_dir_all(runbooks_dir)?;
    runbook.save(&path)?;

    println!(
        "{} Created runbook '{}' at {}",
        "OK".green().bold(),
        name.cyan(),
        path.display()
    );
    Ok(())
}

fn edit_runbook(name: &str, runbooks_dir: &std::path::Path) -> Result<()> {
    let path = runbook_path(name, runbooks_dir);
    if !path.exists() {
        bail!("Runbook '{}' not found.", name);
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to open editor '{}'", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    // Validate the edited file
    match Runbook::from_file(&path) {
        Ok(_) => println!(
            "{} Runbook '{}' saved and validated.",
            "OK".green().bold(),
            name.cyan()
        ),
        Err(e) => println!(
            "{} Runbook saved but has parse errors: {}",
            "WARNING".yellow().bold(),
            e
        ),
    }
    Ok(())
}

fn delete_runbook(name: &str, runbooks_dir: &std::path::Path, auto_confirm: bool) -> Result<()> {
    let path = runbook_path(name, runbooks_dir);
    if !path.exists() {
        bail!("Runbook '{}' not found.", name);
    }

    if !auto_confirm {
        let confirmed = Confirm::new()
            .with_prompt(format!("Delete runbook '{}'?", name))
            .default(false)
            .interact()?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    std::fs::remove_file(&path)?;
    println!("{} Deleted runbook '{}'.", "OK".green().bold(), name.cyan());
    Ok(())
}

fn list_runbooks(runbooks_dir: &std::path::Path) -> Result<()> {
    let runbooks = Runbook::list_runbooks(runbooks_dir)?;

    if runbooks.is_empty() {
        println!("{}", "No runbooks found.".dimmed());
        println!("Create one with: {}", "ds runbook new <name>".cyan());
        return Ok(());
    }

    println!("{}\n", "Runbooks".bold().underline());

    for rb in &runbooks {
        let tags_str = if rb.tags.is_empty() {
            String::new()
        } else {
            format!("  [{}]", rb.tags.join(", ").dimmed())
        };

        println!("  {}  {}{}", rb.name.cyan().bold(), rb.title, tags_str);
        if !rb.description.is_empty() {
            println!("    {}", rb.description.dimmed());
        }
    }

    println!("\n  {} runbook(s) total", runbooks.len().to_string().bold());
    Ok(())
}

fn show_runbook(name: &str, runbooks_dir: &std::path::Path) -> Result<()> {
    let rb = load_runbook(name, runbooks_dir)?;

    println!();
    println!("  {} {}", "Runbook:".bold(), rb.runbook.title.cyan().bold());
    println!("  {} {}", "Name:".bold(), rb.runbook.name);
    if !rb.runbook.description.is_empty() {
        println!("  {} {}", "Description:".bold(), rb.runbook.description);
    }
    println!("  {} {}", "Author:".bold(), rb.runbook.author);
    println!("  {} {}", "Version:".bold(), rb.runbook.version);
    if !rb.runbook.tags.is_empty() {
        println!("  {} {}", "Tags:".bold(), rb.runbook.tags.join(", "));
    }
    if let Some(ref time) = rb.runbook.estimated_time {
        println!("  {} {}", "Estimated time:".bold(), time);
    }
    if !rb.runbook.requires.is_empty() {
        println!(
            "  {} {}",
            "Requires:".bold(),
            rb.runbook.requires.join(", ")
        );
    }

    println!();
    println!("  {}", "Steps:".bold().underline());
    println!();

    for (i, step) in rb.steps.iter().enumerate() {
        let confirm_badge = if step.confirm {
            " [confirm]".yellow().to_string()
        } else {
            String::new()
        };
        let bg_badge = if step.background {
            " [background]".blue().to_string()
        } else {
            String::new()
        };

        println!(
            "  {}. {}{}{}",
            (i + 1).to_string().bold(),
            step.title,
            confirm_badge,
            bg_badge
        );
        println!("     {} {}", "$".dimmed(), step.command.cyan());
        if !step.description.is_empty() {
            println!("     {}", step.description.dimmed());
        }
        if !step.variables.is_empty() {
            println!("     Variables: {}", step.variables.join(", ").yellow());
        }
        println!();
    }
    Ok(())
}

fn run_runbook(
    name: &str,
    runbooks_dir: &std::path::Path,
    from_step: Option<usize>,
    var: Vec<(String, String)>,
    auto_confirm: bool,
) -> Result<()> {
    let rb = load_runbook(name, runbooks_dir)?;

    println!(
        "\n{} Running runbook: {}\n",
        ">>>".green().bold(),
        rb.runbook.title.cyan().bold()
    );

    // Collect any required variables that weren't passed via --var
    let mut variables: HashMap<String, String> = var.into_iter().collect();
    for step in &rb.steps {
        for var_name in &step.variables {
            if !variables.contains_key(var_name) {
                let value: String = Input::new()
                    .with_prompt(format!("Variable '{}'", var_name))
                    .interact_text()?;
                variables.insert(var_name.clone(), value);
            }
        }
    }

    let options = ExecutionOptions {
        auto_confirm,
        dry_run: false,
        from_step,
        variables,
    };

    let confirm_fn = |step: &ds_core::runbook::RunbookStep, command: &str| -> Result<bool> {
        println!("  {} {}", "Step:".bold(), step.title);
        println!("  {} {}", "$".dimmed(), command);
        let confirmed = Confirm::new()
            .with_prompt("  Execute this step?")
            .default(true)
            .interact()?;
        Ok(confirmed)
    };

    let results = RunbookExecutor::execute(&rb, &options, &confirm_fn)?;

    println!("\n{}", "Results:".bold().underline());
    let mut all_ok = true;
    for res in &results {
        let icon = if res.skipped {
            "SKIP".yellow().to_string()
        } else if res.success {
            "OK".green().to_string()
        } else {
            all_ok = false;
            "FAIL".red().to_string()
        };
        println!("  [{}] Step {}: {}", icon, res.step_index + 1, res.title);
        if !res.output.is_empty() && !res.skipped {
            // Show first few lines of output
            for line in res.output.lines().take(5) {
                println!("       {}", line.dimmed());
            }
        }
    }

    if all_ok {
        println!("\n{} Runbook completed successfully.", "OK".green().bold());
    } else {
        println!("\n{} Runbook completed with errors.", "ERROR".red().bold());
    }
    Ok(())
}

fn start_recording(name: Option<String>) -> Result<()> {
    let recorder = RunbookRecorder::new();
    recorder.start(name.clone());

    let label = name.as_deref().unwrap_or("unnamed");
    println!(
        "{} Recording started (name: {})",
        "REC".magenta().bold(),
        label.cyan()
    );
    println!(
        "Run commands in your shell, then stop recording with: {}",
        "ds runbook stop".cyan()
    );

    // Persist recording state to a marker file so `stop` knows we are recording
    let marker = ConfigLoader::data_dir().join(".recording");
    let marker_content = name.unwrap_or_default();
    std::fs::write(&marker, marker_content)?;

    Ok(())
}

fn stop_recording(runbooks_dir: &std::path::Path) -> Result<()> {
    let marker = ConfigLoader::data_dir().join(".recording");
    if !marker.exists() {
        bail!(
            "No recording in progress. Start one with: {}",
            "ds runbook record [name]".cyan()
        );
    }

    let name_from_file = std::fs::read_to_string(&marker)?;
    let name = if name_from_file.trim().is_empty() {
        None
    } else {
        Some(name_from_file.trim().to_string())
    };

    let recorder = RunbookRecorder::new();
    recorder.start(name);

    // Since the recorder is ephemeral and commands are tracked via shell hooks,
    // we build a runbook from recent command history instead.
    let runbook = recorder.stop()?;

    if runbook.steps.is_empty() {
        println!(
            "{} No commands were recorded. The recording session was empty.",
            "WARNING".yellow().bold()
        );
    } else {
        std::fs::create_dir_all(runbooks_dir)?;
        let path = runbooks_dir.join(format!("{}.toml", runbook.runbook.name));
        runbook.save(&path)?;
        println!(
            "{} Recording stopped. Saved runbook '{}' ({} steps) to {}",
            "OK".green().bold(),
            runbook.runbook.name.cyan(),
            runbook.steps.len(),
            path.display()
        );
    }

    // Clean up marker
    std::fs::remove_file(&marker).ok();
    Ok(())
}

async fn generate_runbook(
    description: &str,
    config: &DeftShellConfig,
    runbooks_dir: &std::path::Path,
) -> Result<()> {
    println!(
        "{} Generating runbook from: {}",
        "AI".magenta().bold(),
        description.cyan()
    );

    let gateway = ds_core::ai::gateway::AiGateway::new(&config.ai);
    let request = ds_core::ai::gateway::AiRequest {
        system_prompt: Some(
            "You are a DevOps runbook generator. Generate a runbook in TOML format. \
             The runbook should use this schema:\n\
             [runbook]\n\
             name = \"...\"\n\
             title = \"...\"\n\
             description = \"...\"\n\
             author = \"ai\"\n\
             tags = [...]\n\n\
             [[steps]]\n\
             title = \"...\"\n\
             command = \"...\"\n\
             confirm = true\n\n\
             Output ONLY the TOML content, nothing else."
                .to_string(),
        ),
        messages: vec![ds_core::ai::gateway::ChatMessage {
            role: ds_core::ai::gateway::MessageRole::User,
            content: format!("Generate a runbook for: {}", description),
        }],
        max_tokens: Some(2048),
        temperature: Some(0.3),
        stream: false,
    };

    let response = gateway.complete(&request).await?;

    // Try to parse the generated TOML
    let toml_content = response
        .content
        .trim()
        .trim_start_matches("```toml")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match Runbook::parse_toml(toml_content) {
        Ok(runbook) => {
            let save = Confirm::new()
                .with_prompt("Save this generated runbook?")
                .default(true)
                .interact()?;

            if save {
                std::fs::create_dir_all(runbooks_dir)?;
                let path = runbooks_dir.join(format!("{}.toml", runbook.runbook.name));
                runbook.save(&path)?;
                println!(
                    "{} Saved generated runbook to {}",
                    "OK".green().bold(),
                    path.display()
                );
            }
        }
        Err(e) => {
            println!(
                "{} AI generated invalid TOML: {}",
                "WARNING".yellow().bold(),
                e
            );
            println!("\nRaw output:\n{}", toml_content.dimmed());
        }
    }

    Ok(())
}

async fn search_registry(query: &str) -> Result<()> {
    println!("Searching runbook registry for '{}'...\n", query.cyan());

    let registry = RunbookRegistry::new(None);
    let results = registry.search(query).await?;

    if results.is_empty() {
        println!("{}", "No runbooks found matching your query.".dimmed());
        return Ok(());
    }

    for entry in &results {
        println!(
            "  {}/{} v{} ({} stars, {} downloads)",
            entry.author.dimmed(),
            entry.name.cyan().bold(),
            entry.version,
            entry.stars.to_string().yellow(),
            entry.downloads
        );
        println!("    {}", entry.description);
        if !entry.tags.is_empty() {
            println!("    Tags: {}", entry.tags.join(", ").dimmed());
        }
        println!();
    }

    println!(
        "Install with: {}",
        "ds runbook install <author>/<name>".cyan()
    );
    Ok(())
}

async fn install_from_registry(spec: &str, runbooks_dir: &std::path::Path) -> Result<()> {
    let parts: Vec<&str> = spec.splitn(2, '/').collect();
    if parts.len() != 2 {
        bail!("Invalid spec '{}'. Expected format: author/name", spec);
    }
    let (author, name) = (parts[0], parts[1]);

    println!("Installing runbook {}/{}...", author.dimmed(), name.cyan());

    let registry = RunbookRegistry::new(None);
    let runbook = registry.install(author, name).await?;

    std::fs::create_dir_all(runbooks_dir)?;
    let path = runbooks_dir.join(format!("{}.toml", runbook.runbook.name));
    runbook.save(&path)?;

    println!(
        "{} Installed '{}' to {}",
        "OK".green().bold(),
        runbook.runbook.title.cyan(),
        path.display()
    );
    Ok(())
}

fn publish_runbook(name: &str) -> Result<()> {
    println!(
        "{} Publishing runbooks to the registry is not yet available.",
        "INFO".blue().bold()
    );
    println!(
        "Runbook '{}' is ready locally. Community publishing will be available in a future release.",
        name.cyan()
    );
    Ok(())
}

async fn show_trending() -> Result<()> {
    println!("{}\n", "Trending Runbooks".bold().underline());

    let registry = RunbookRegistry::new(None);
    let entries = registry.trending(10).await?;

    if entries.is_empty() {
        println!("{}", "No trending runbooks available.".dimmed());
        return Ok(());
    }

    for (i, entry) in entries.iter().enumerate() {
        println!(
            "  {}. {}/{} ({} stars)",
            (i + 1).to_string().bold(),
            entry.author.dimmed(),
            entry.name.cyan().bold(),
            entry.stars.to_string().yellow()
        );
        println!("     {}", entry.description);
    }
    Ok(())
}
