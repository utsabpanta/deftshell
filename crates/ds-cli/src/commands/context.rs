use anyhow::{bail, Result};
use colored::Colorize;
use ds_core::config::DeftShellConfig;
use ds_core::context::{detect_workspace, list_workspace_packages, ContextDetector, StackProfile};

use crate::{ContextAction, WorkspaceAction};

/// `ds context [action]` - show, refresh, export, or diff project context
pub fn run(
    action: Option<ContextAction>,
    detect: bool,
    quiet: bool,
    _config: &DeftShellConfig,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    match action {
        Some(ContextAction::Refresh) => {
            let profile = ContextDetector::detect(&cwd)?;
            if !quiet {
                println!("{}", "Context refreshed.".green());
                print_profile(&profile);
            }
        }
        Some(ContextAction::Export) => {
            let profile = ContextDetector::detect(&cwd)?;
            let json = serde_json::to_string_pretty(&profile)?;
            println!("{}", json);
        }
        Some(ContextAction::Diff) => {
            // Detect current context and show a summary; a full diff would
            // compare against the cached version, but for now we display the
            // current snapshot.
            let profile = ContextDetector::detect(&cwd)?;
            println!("{}", "Current detected context:".cyan().bold());
            print_profile(&profile);
            println!();
            println!(
                "{}",
                "(Diff against cached context is not yet implemented.)".dimmed()
            );
        }
        None => {
            if detect {
                // Silent detection mode used by shell hooks
                let profile = ContextDetector::detect(&cwd)?;
                if !quiet {
                    if let Some(ref lang) = profile.stack.primary_language {
                        print!("{}", lang);
                    }
                }
            } else {
                let profile = ContextDetector::detect(&cwd)?;
                print_profile(&profile);
            }
        }
    }

    Ok(())
}

/// `ds scripts` - list project scripts from detected context
pub fn run_scripts(_config: &DeftShellConfig) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let profile = ContextDetector::detect(&cwd)?;

    // Collect scripts from the detected profile
    let mut scripts = profile.scripts.clone();

    // Also try to detect scripts from Makefile if present
    let makefile_scripts = detect_makefile_targets(&cwd);
    for (name, cmd) in &makefile_scripts {
        scripts.entry(name.clone()).or_insert_with(|| cmd.clone());
    }

    // Also load scripts from .deftshell.toml project config
    if let Ok(Some(project_config)) = ds_core::config::ConfigLoader::load_project_config(&cwd) {
        for (name, cmd) in &project_config.scripts {
            scripts.entry(name.clone()).or_insert_with(|| cmd.clone());
        }
    }

    if scripts.is_empty() {
        println!("{}", "No scripts found in this project.".yellow());
        println!(
            "{}",
            "Tip: Add scripts to package.json, Makefile, or .deftshell.toml".dimmed()
        );
        return Ok(());
    }

    println!("{}", "Available scripts:".cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());

    // Sort scripts by name for consistent output
    let mut sorted: Vec<_> = scripts.iter().collect();
    sorted.sort_by_key(|(name, _)| (*name).clone());

    for (name, cmd) in sorted {
        println!("  {}  {}", name.green().bold(), cmd.dimmed());
    }

    println!();
    println!("Run a script with: {}", "ds run <script>".cyan());

    Ok(())
}

/// `ds run <script>` - run a project script
pub fn run_script(script: &str, _config: &DeftShellConfig) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let profile = ContextDetector::detect(&cwd)?;

    // Look up the script in detected scripts
    let mut scripts = profile.scripts.clone();

    // Merge Makefile targets
    let makefile_scripts = detect_makefile_targets(&cwd);
    for (name, cmd) in &makefile_scripts {
        scripts.entry(name.clone()).or_insert_with(|| cmd.clone());
    }

    // Merge .deftshell.toml scripts
    if let Ok(Some(project_config)) = ds_core::config::ConfigLoader::load_project_config(&cwd) {
        for (name, cmd) in &project_config.scripts {
            scripts.entry(name.clone()).or_insert_with(|| cmd.clone());
        }
    }

    // Determine how to run the script. Scripts from package.json should be
    // executed through the package manager (npm run, yarn, pnpm run, etc.)
    // because their raw commands reference binaries in node_modules/.bin/
    // which aren't on the system PATH.
    let command = if scripts.contains_key(script) && cwd.join("package.json").exists() {
        let pm = profile.stack.package_manager.as_deref().unwrap_or("npm");
        // yarn uses `yarn <script>`, npm/pnpm use `<pm> run <script>`
        if pm == "yarn" {
            format!("yarn {}", script)
        } else {
            format!("{} run {}", pm, script)
        }
    } else if scripts.contains_key(script) {
        // Non-JS script (from Makefile or .deftshell.toml) — run directly
        scripts.get(script).unwrap().clone()
    } else {
        // Script not found in detected scripts — try common fallbacks
        if cwd.join("Makefile").exists() || cwd.join("makefile").exists() {
            format!("make {}", script)
        } else if cwd.join("package.json").exists() {
            let pm = profile.stack.package_manager.as_deref().unwrap_or("npm");
            if pm == "yarn" {
                format!("yarn {}", script)
            } else {
                format!("{} run {}", pm, script)
            }
        } else {
            bail!(
                "Script '{}' not found. Run `ds scripts` to see available scripts.",
                script
            );
        }
    };

    println!("{} {}", "Running:".green().bold(), command.cyan());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .current_dir(&cwd)
        .status()?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("Script '{}' exited with code {}", script, code);
    }

    Ok(())
}

/// `ds workspace [action]` - workspace list or navigate
pub fn run_workspace(action: Option<WorkspaceAction>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let workspace_info = match detect_workspace(&cwd) {
        Some(info) => info,
        None => {
            println!(
                "{}",
                "No workspace/monorepo detected in the current directory.".yellow()
            );
            println!(
                "{}",
                "Supported workspace types: npm, yarn, pnpm, lerna, nx, turborepo, cargo".dimmed()
            );
            return Ok(());
        }
    };

    match action {
        Some(WorkspaceAction::List) | None => {
            println!(
                "{} {} workspace",
                "Detected:".cyan().bold(),
                workspace_info.workspace_type.to_string().green().bold()
            );
            println!(
                "  Root: {}",
                workspace_info.root.display().to_string().dimmed()
            );
            println!();

            let packages = list_workspace_packages(&workspace_info);
            if packages.is_empty() {
                println!("{}", "No packages found.".yellow());
            } else {
                println!(
                    "{} ({} packages):",
                    "Packages".cyan().bold(),
                    packages.len()
                );
                println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
                for pkg in &packages {
                    let relative_path = pkg
                        .path
                        .strip_prefix(&workspace_info.root)
                        .unwrap_or(&pkg.path);
                    println!(
                        "  {}  {}",
                        pkg.name.green().bold(),
                        relative_path.display().to_string().dimmed()
                    );
                }
            }
        }
    }

    Ok(())
}

/// `ds env` - display environment context
pub fn run_env(config: &DeftShellConfig) -> Result<()> {
    let cwd = std::env::current_dir()?;

    println!("{}", "Environment Context".cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());

    // Shell info
    println!();
    println!("{}", "Shell:".bold());
    println!(
        "  Type:     {}",
        std::env::var("SHELL")
            .unwrap_or_else(|_| "unknown".into())
            .green()
    );
    println!("  CWD:      {}", cwd.display().to_string().dimmed());
    if let Ok(term) = std::env::var("TERM") {
        println!("  Terminal:  {}", term.dimmed());
    }

    // Runtime versions
    println!();
    println!("{}", "Runtimes:".bold());
    print_runtime_version("node", &["--version"]);
    print_runtime_version("python3", &["--version"]);
    print_runtime_version("python", &["--version"]);
    print_runtime_version("ruby", &["--version"]);
    print_runtime_version("go", &["version"]);
    print_runtime_version("rustc", &["--version"]);
    print_runtime_version("java", &["-version"]);
    print_runtime_version("deno", &["--version"]);
    print_runtime_version("bun", &["--version"]);

    // Environment variables of interest
    println!();
    println!("{}", "Environment:".bold());

    let env_vars = [
        "NODE_ENV",
        "RAILS_ENV",
        "FLASK_ENV",
        "APP_ENV",
        "ENVIRONMENT",
        "AWS_PROFILE",
        "AWS_REGION",
        "KUBECONFIG",
        "DOCKER_HOST",
        "VIRTUAL_ENV",
        "CONDA_DEFAULT_ENV",
        "GOPATH",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "NVM_DIR",
        "PYENV_ROOT",
    ];

    let mut any_set = false;
    for var in &env_vars {
        if let Ok(val) = std::env::var(var) {
            println!("  {}={}", var.green(), val.dimmed());
            any_set = true;
        }
    }
    if !any_set {
        println!("  {}", "(no notable environment variables set)".dimmed());
    }

    // Git context
    println!();
    println!("{}", "Git:".bold());
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&cwd)
        .output()
    {
        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("  Root:     {}", root.dimmed());

            if let Ok(branch_output) = std::process::Command::new("git")
                .args(["branch", "--show-current"])
                .current_dir(&cwd)
                .output()
            {
                if branch_output.status.success() {
                    let branch = String::from_utf8_lossy(&branch_output.stdout)
                        .trim()
                        .to_string();
                    println!("  Branch:   {}", branch.green());
                }
            }

            if let Ok(remote_output) = std::process::Command::new("git")
                .args(["remote", "-v"])
                .current_dir(&cwd)
                .output()
            {
                if remote_output.status.success() {
                    let remotes = String::from_utf8_lossy(&remote_output.stdout)
                        .trim()
                        .to_string();
                    if let Some(first_line) = remotes.lines().next() {
                        println!("  Remote:   {}", first_line.dimmed());
                    }
                }
            }
        } else {
            println!("  {}", "(not a git repository)".dimmed());
        }
    } else {
        println!("  {}", "(git not available)".dimmed());
    }

    // AI provider status
    println!();
    println!("{}", "AI Provider:".bold());
    println!("  Default:  {}", config.ai.default_provider.green());
    if config.ai.privacy_mode {
        println!("  Mode:     {}", "privacy".yellow().bold());
    }
    if let Some(ref fallback) = config.ai.fallback_provider {
        println!("  Fallback: {}", fallback.dimmed());
    }

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Pretty-print a detected StackProfile.
fn print_profile(profile: &StackProfile) {
    println!("{}", "Project Context".cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());

    // Project info
    if !profile.project.name.is_empty() {
        println!("  Project:        {}", profile.project.name.green().bold());
    }
    println!("  Root:           {}", profile.project.root.cyan());
    if let Some(ref vcs) = profile.project.vcs {
        println!("  VCS:            {}", vcs.dimmed());
    }

    // Stack info
    println!();
    println!("{}", "  Stack:".bold());
    if let Some(ref lang) = profile.stack.primary_language {
        println!("    Language:     {}", lang.green());
    }
    if let Some(ref runtime) = profile.stack.runtime {
        let version_str = profile
            .stack
            .runtime_version
            .as_deref()
            .map(|v| format!(" ({})", v))
            .unwrap_or_default();
        println!(
            "    Runtime:      {}{}",
            runtime.green(),
            version_str.dimmed()
        );
    }
    if let Some(ref framework) = profile.stack.framework {
        let version_str = profile
            .stack
            .framework_version
            .as_deref()
            .map(|v| format!(" ({})", v))
            .unwrap_or_default();
        println!(
            "    Framework:    {}{}",
            framework.green(),
            version_str.dimmed()
        );
    }
    if let Some(ref pm) = profile.stack.package_manager {
        println!("    Pkg Manager:  {}", pm.dimmed());
    }
    if let Some(ref runner) = profile.stack.test_runner {
        println!("    Test Runner:  {}", runner.dimmed());
    }
    if let Some(ref linter) = profile.stack.linter {
        println!("    Linter:       {}", linter.dimmed());
    }
    if let Some(ref formatter) = profile.stack.formatter {
        println!("    Formatter:    {}", formatter.dimmed());
    }
    if let Some(ref bundler) = profile.stack.bundler {
        println!("    Bundler:      {}", bundler.dimmed());
    }

    // Infrastructure
    if profile.infrastructure.containerized
        || profile.infrastructure.orchestration.is_some()
        || profile.infrastructure.cloud_provider.is_some()
        || profile.infrastructure.ci_cd.is_some()
    {
        println!();
        println!("{}", "  Infrastructure:".bold());
        if profile.infrastructure.containerized {
            println!("    Container:    {}", "Docker".green());
        }
        if let Some(ref orch) = profile.infrastructure.orchestration {
            println!("    Orchestration:{}", format!(" {}", orch).green());
        }
        if let Some(ref cloud) = profile.infrastructure.cloud_provider {
            println!("    Cloud:        {}", cloud.green());
        }
        if let Some(ref ci) = profile.infrastructure.ci_cd {
            println!("    CI/CD:        {}", ci.green());
        }
    }

    // Services
    if profile.services.database.is_some()
        || profile.services.cache.is_some()
        || profile.services.message_queue.is_some()
    {
        println!();
        println!("{}", "  Services:".bold());
        if let Some(ref db) = profile.services.database {
            println!("    Database:     {}", db.green());
        }
        if let Some(ref cache) = profile.services.cache {
            println!("    Cache:        {}", cache.green());
        }
        if let Some(ref mq) = profile.services.message_queue {
            println!("    Queue:        {}", mq.green());
        }
    }

    // Scripts summary
    if !profile.scripts.is_empty() {
        println!();
        println!(
            "  {} {} scripts available (run `ds scripts` to list)",
            "Scripts:".bold(),
            profile.scripts.len()
        );
    }
}

/// Detect Makefile targets by parsing a Makefile for target definitions.
fn detect_makefile_targets(dir: &std::path::Path) -> Vec<(String, String)> {
    let mut targets = Vec::new();

    let makefile_path = if dir.join("Makefile").exists() {
        dir.join("Makefile")
    } else if dir.join("makefile").exists() {
        dir.join("makefile")
    } else {
        return targets;
    };

    let contents = match std::fs::read_to_string(&makefile_path) {
        Ok(c) => c,
        Err(_) => return targets,
    };

    for line in contents.lines() {
        // Match lines like `target: dependencies`
        // Skip lines that start with whitespace (recipe lines), comments, or
        // variable assignments.
        if line.starts_with('\t')
            || line.starts_with(' ')
            || line.starts_with('#')
            || line.starts_with('.')
            || line.contains('=')
        {
            continue;
        }

        if let Some(colon_pos) = line.find(':') {
            let target = line[..colon_pos].trim();
            // Skip phony declarations and empty targets
            if !target.is_empty()
                && !target.contains(' ')
                && !target.starts_with('$')
                && target != ".PHONY"
                && target != ".DEFAULT"
                && target != ".SUFFIXES"
            {
                targets.push((target.to_string(), format!("make {}", target)));
            }
        }
    }

    targets
}

/// Print the version of a runtime tool, if it is available on PATH.
fn print_runtime_version(cmd: &str, args: &[&str]) {
    if let Ok(output) = std::process::Command::new(cmd)
        .args(args)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
    {
        if output.status.success() {
            // Some tools (e.g. java) print to stderr
            let out = if output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr).trim().to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            };
            // Take only the first line for brevity
            if let Some(first_line) = out.lines().next() {
                println!(
                    "  {:<10}{}",
                    format!("{}:", cmd).green(),
                    first_line.dimmed()
                );
            }
        }
    }
}
