use anyhow::Result;
use colored::Colorize;
use ds_core::config::ConfigLoader;

/// `ds doctor` - diagnose issues with DeftShell installation and environment
pub fn run(verbose: bool) -> Result<()> {
    println!("{}", "DeftShell Doctor".cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
    println!();

    let mut pass_count: u32 = 0;
    let mut fail_count: u32 = 0;
    let mut warn_count: u32 = 0;

    // ── Check: ds binary in PATH ────────────────────────────────────────

    let ds_in_path = which_exists("ds");
    print_check(
        "ds binary in PATH",
        ds_in_path,
        if ds_in_path {
            None
        } else {
            Some("Add the ds binary location to your PATH")
        },
    );
    count_result(ds_in_path, &mut pass_count, &mut fail_count);

    // ── Check: Shell integration configured ─────────────────────────────

    let shell_configured = check_shell_init();
    print_check(
        "Shell integration configured",
        shell_configured,
        if shell_configured {
            None
        } else {
            Some("Add `eval \"$(ds init zsh)\"` (or bash/fish) to your shell rc file")
        },
    );
    count_result(shell_configured, &mut pass_count, &mut fail_count);

    // ── Check: Git available ────────────────────────────────────────────

    let git_ok = check_command_available("git", &["--version"], verbose);
    print_check(
        "Git available",
        git_ok,
        if git_ok {
            None
        } else {
            Some("Install git: https://git-scm.com/downloads")
        },
    );
    count_result(git_ok, &mut pass_count, &mut fail_count);

    // ── Check: Node.js available ────────────────────────────────────────

    let node_ok = check_command_available("node", &["--version"], verbose);
    print_check(
        "Node.js available",
        node_ok,
        if node_ok {
            None
        } else {
            Some("Install Node.js: https://nodejs.org/ (optional, needed for JS/TS projects)")
        },
    );
    if node_ok {
        pass_count += 1;
    } else {
        warn_count += 1;
    }

    // ── Check: AI providers ─────────────────────────────────────────────

    println!();
    println!("{}", "AI Providers:".bold());

    // Check Ollama (default provider)
    let ollama_ok = check_ollama_available(verbose);
    print_check(
        "  Ollama (local)",
        ollama_ok,
        if ollama_ok {
            None
        } else {
            Some("Install Ollama: https://ollama.ai/download")
        },
    );
    if ollama_ok {
        pass_count += 1;
    } else {
        warn_count += 1;
    }

    // Check OpenAI API key
    let openai_ok = std::env::var("OPENAI_API_KEY").is_ok();
    print_check(
        "  OpenAI API key",
        openai_ok,
        if openai_ok {
            None
        } else {
            Some("Set OPENAI_API_KEY environment variable (optional)")
        },
    );
    if openai_ok {
        pass_count += 1;
    } else {
        warn_count += 1;
    }

    // Check Anthropic API key
    let anthropic_ok = std::env::var("ANTHROPIC_API_KEY").is_ok();
    print_check(
        "  Anthropic API key",
        anthropic_ok,
        if anthropic_ok {
            None
        } else {
            Some("Set ANTHROPIC_API_KEY environment variable (optional)")
        },
    );
    if anthropic_ok {
        pass_count += 1;
    } else {
        warn_count += 1;
    }

    // Check Google Gemini API key
    let gemini_ok =
        std::env::var("GOOGLE_API_KEY").is_ok() || std::env::var("GEMINI_API_KEY").is_ok();
    print_check(
        "  Google Gemini API key",
        gemini_ok,
        if gemini_ok {
            None
        } else {
            Some("Set GOOGLE_API_KEY or GEMINI_API_KEY environment variable (optional)")
        },
    );
    if gemini_ok {
        pass_count += 1;
    } else {
        warn_count += 1;
    }

    // ── Check: Database accessible ──────────────────────────────────────

    println!();
    println!("{}", "Storage:".bold());

    let db_path = ConfigLoader::db_path();
    let db_ok = check_database(&db_path, verbose);
    let db_hint = format!("Check permissions on {}", db_path.display());
    print_check(
        "Database accessible",
        db_ok,
        if db_ok { None } else { Some(&db_hint) },
    );
    count_result(db_ok, &mut pass_count, &mut fail_count);

    if verbose {
        println!("  Path: {}", db_path.display().to_string().dimmed());
    }

    // ── Check: Data directory ───────────────────────────────────────────

    let data_dir = ConfigLoader::data_dir();
    let data_dir_ok = data_dir.exists() && data_dir.is_dir();
    print_check(
        "Data directory exists",
        data_dir_ok,
        if data_dir_ok {
            None
        } else {
            Some("Run `ds init <shell>` to create the data directory")
        },
    );
    count_result(data_dir_ok, &mut pass_count, &mut fail_count);

    if verbose {
        println!("  Path: {}", data_dir.display().to_string().dimmed());
    }

    // ── Check: Config file ──────────────────────────────────────────────

    let config_path = ConfigLoader::user_config_path();
    let config_ok = config_path.as_ref().map(|p| p.exists()).unwrap_or(false);
    print_check(
        "User config file",
        config_ok,
        if config_ok {
            None
        } else {
            Some("Config will use defaults. Create ~/.deftshell/config.toml to customize")
        },
    );
    if config_ok {
        pass_count += 1;
    } else {
        warn_count += 1;
    }

    if verbose {
        if let Some(ref path) = config_path {
            println!("  Path: {}", path.display().to_string().dimmed());
        }
    }

    // ── Check: Plugins directory ────────────────────────────────────────

    let plugins_dir = data_dir.join("plugins");
    let plugins_ok = plugins_dir.exists() && plugins_dir.is_dir();
    print_check(
        "Plugins directory exists",
        plugins_ok,
        if plugins_ok {
            None
        } else {
            Some("Plugins directory will be created on first plugin install")
        },
    );
    if plugins_ok {
        pass_count += 1;
    } else {
        warn_count += 1;
    }

    if verbose {
        println!("  Path: {}", plugins_dir.display().to_string().dimmed());

        // List installed plugins if any
        if plugins_ok {
            if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
                let plugins: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .collect();
                if plugins.is_empty() {
                    println!("  {}", "(no plugins installed)".dimmed());
                } else {
                    for p in &plugins {
                        println!("  - {}", p.dimmed());
                    }
                }
            }
        }
    }

    // ── Summary ─────────────────────────────────────────────────────────

    println!();
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
    println!(
        "  {} passed, {} warnings, {} failed",
        pass_count.to_string().green().bold(),
        warn_count.to_string().yellow().bold(),
        fail_count.to_string().red().bold()
    );

    if fail_count > 0 {
        println!();
        println!(
            "{}",
            "Some checks failed. Fix the issues above for the best experience.".red()
        );
    } else if warn_count > 0 {
        println!();
        println!(
            "{}",
            "All critical checks passed. Optional features noted above.".green()
        );
    } else {
        println!();
        println!(
            "{}",
            "Everything looks great! DeftShell is fully configured."
                .green()
                .bold()
        );
    }

    Ok(())
}

// ── Helper functions ────────────────────────────────────────────────────

/// Print a check result line with a pass/fail icon.
fn print_check(label: &str, passed: bool, hint: Option<&str>) {
    if passed {
        println!("  {} {}", "[OK]".green().bold(), label);
    } else {
        println!("  {} {}", "[!!]".red().bold(), label);
        if let Some(h) = hint {
            println!("      {}", h.yellow());
        }
    }
}

/// Update pass/fail counters.
fn count_result(passed: bool, pass: &mut u32, fail: &mut u32) {
    if passed {
        *pass += 1;
    } else {
        *fail += 1;
    }
}

/// Check if a command exists on PATH by running `which`.
fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if a command is available and optionally print its version.
fn check_command_available(cmd: &str, args: &[&str], verbose: bool) -> bool {
    match std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => {
            if verbose {
                let version_str = if output.stdout.is_empty() {
                    String::from_utf8_lossy(&output.stderr).trim().to_string()
                } else {
                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                };
                if let Some(first_line) = version_str.lines().next() {
                    println!("      {}", first_line.dimmed());
                }
            }
            true
        }
        _ => false,
    }
}

/// Check if Ollama is running and accessible.
fn check_ollama_available(verbose: bool) -> bool {
    // First check if the binary exists
    if !which_exists("ollama") {
        return false;
    }

    // Try to list models (checks if the server is running)
    match std::process::Command::new("ollama")
        .arg("list")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => {
            if verbose {
                let list = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let model_count = list.lines().count().saturating_sub(1); // Subtract header line
                println!(
                    "      {} model{} available",
                    model_count,
                    if model_count == 1 { "" } else { "s" }
                );
            }
            true
        }
        _ => false,
    }
}

/// Check if the DeftShell database can be opened.
fn check_database(path: &std::path::Path, verbose: bool) -> bool {
    match ds_core::storage::Database::open(path) {
        Ok(_) => {
            if verbose {
                if let Ok(metadata) = std::fs::metadata(path) {
                    let size_kb = metadata.len() / 1024;
                    println!("      Size: {} KB", size_kb);
                }
            }
            true
        }
        Err(e) => {
            if verbose {
                println!("      Error: {}", e.to_string().dimmed());
            }
            false
        }
    }
}

/// Check if shell init is configured by looking for ds init calls in shell rc files.
fn check_shell_init() -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };

    let rc_files = [
        home.join(".zshrc"),
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".config/fish/config.fish"),
    ];

    for rc in &rc_files {
        if let Ok(contents) = std::fs::read_to_string(rc) {
            if contents.contains("ds init") || contents.contains("deftshell") {
                return true;
            }
        }
    }

    false
}
