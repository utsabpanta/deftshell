use anyhow::Result;
use colored::Colorize;

use ds_core::config::{ConfigLoader, DeftShellConfig};
use ds_core::storage::db::{Database, UsagePeriod};

/// Entry point for `ds stats [period] [--format fmt]`.
pub fn run(period: Option<&str>, format: Option<&str>, _config: &DeftShellConfig) -> Result<()> {
    let db = Database::open(&ConfigLoader::db_path())?;
    let cwd = std::env::current_dir()?;
    let cwd_str = cwd.to_string_lossy();

    // If a format is specified, export data
    if let Some(fmt) = format {
        return export_stats(&db, period.unwrap_or("all"), fmt, &cwd_str);
    }

    // Display summary
    display_summary(&db, period, &cwd_str)
}

/// `ds usage` - show AI usage statistics and estimated costs.
pub fn run_usage(config: &DeftShellConfig) -> Result<()> {
    let db = Database::open(&ConfigLoader::db_path())?;

    println!("\n{}\n", "AI Token Usage".bold().underline());
    println!(
        "  {}\n",
        "Token counts are estimated (~4 chars/token) for streaming requests.".dimmed()
    );

    // Show usage for multiple periods
    let periods = [
        ("Today", UsagePeriod::Today),
        ("This Week", UsagePeriod::Week),
        ("This Month", UsagePeriod::Month),
        ("All Time", UsagePeriod::All),
    ];

    let mut any_usage = false;
    for (label, period) in &periods {
        let usage = db.get_ai_usage(*period)?;

        if usage.total_tokens_in == 0 && usage.total_tokens_out == 0 {
            continue;
        }

        any_usage = true;
        println!("  {}", label.bold());
        println!(
            "    Tokens in:   {}",
            format_number(usage.total_tokens_in).cyan()
        );
        println!(
            "    Tokens out:  {}",
            format_number(usage.total_tokens_out).cyan()
        );
        if !usage.by_provider.is_empty() {
            println!("    By provider:");
            for (provider, stats) in &usage.by_provider {
                println!(
                    "      {}: {} in / {} out",
                    provider.cyan(),
                    format_number(stats.tokens_in),
                    format_number(stats.tokens_out),
                );
            }
        }
        println!();
    }

    if !any_usage {
        println!("  {}", "No AI usage recorded yet.".dimmed());
        println!(
            "  Try: {}",
            "ds ask \"how do I build this project?\"".cyan()
        );
        println!();
        return Ok(());
    }

    // Show daily token limit info
    let daily = db.get_ai_usage(UsagePeriod::Today)?;
    let limit = config.ai.limits.daily_token_limit;
    let total_today = daily.total_tokens_in + daily.total_tokens_out;
    let pct = if limit > 0 {
        (total_today as f64 / limit as f64 * 100.0) as u64
    } else {
        0
    };

    println!("  {}", "Daily Limit".bold());
    println!(
        "    {} / {} tokens ({}%)",
        format_number(total_today),
        format_number(limit),
        if pct > config.ai.limits.warn_at_percentage as u64 {
            pct.to_string().red().to_string()
        } else {
            pct.to_string().green().to_string()
        }
    );

    let bar_width = 30;
    let filled = ((pct as usize).min(100) * bar_width) / 100;
    let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(bar_width - filled));
    println!(
        "    {}",
        if pct > config.ai.limits.warn_at_percentage as u64 {
            bar.red().to_string()
        } else {
            bar.green().to_string()
        }
    );

    println!();
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn display_summary(db: &Database, period: Option<&str>, cwd: &str) -> Result<()> {
    let period_label = period.unwrap_or("project");

    println!(
        "\n{} ({})\n",
        "Analytics Dashboard".bold().underline(),
        period_label.cyan()
    );

    // Get command stats - use cwd for project scope, "*" for global
    let stats = match period_label {
        "today" | "week" | "all" => db.get_command_stats("*")?,
        _ => db.get_command_stats(cwd)?,
    };

    // Commands overview
    println!("  {}", "Commands".bold());
    println!(
        "    Total:        {}",
        stats.total_commands.to_string().cyan().bold()
    );
    println!(
        "    Unique:       {}",
        stats.unique_commands.to_string().cyan()
    );

    let error_pct = (stats.error_rate * 100.0) as u64;
    let error_str = format!("{}%", error_pct);
    println!(
        "    Error rate:   {}",
        if error_pct > 20 {
            error_str.red().to_string()
        } else if error_pct > 5 {
            error_str.yellow().to_string()
        } else {
            error_str.green().to_string()
        }
    );

    // Most used commands
    if !stats.most_used.is_empty() {
        println!();
        println!("  {}", "Most Used Commands".bold());
        for (i, (cmd, count)) in stats.most_used.iter().take(10).enumerate() {
            let bar_len = if stats.most_used[0].1 > 0 {
                (*count as usize * 20) / stats.most_used[0].1 as usize
            } else {
                0
            };
            let bar = "|".repeat(bar_len.max(1));
            println!(
                "    {:>2}. {:<30} {:>5}  {}",
                i + 1,
                truncate_str(cmd, 30),
                count.to_string().bold(),
                bar.cyan()
            );
        }
    }

    // AI usage
    let usage_period = match period_label {
        "today" => UsagePeriod::Today,
        "week" => UsagePeriod::Week,
        _ => UsagePeriod::All,
    };
    let ai_usage = db.get_ai_usage(usage_period)?;

    if ai_usage.total_tokens_in > 0 || ai_usage.total_tokens_out > 0 {
        println!();
        println!("  {}", "AI Usage".bold());
        println!(
            "    Tokens (in/out): {} / {}",
            format_number(ai_usage.total_tokens_in).cyan(),
            format_number(ai_usage.total_tokens_out).cyan()
        );
        // Cost tracking not yet implemented — token counts are estimates.

        if !ai_usage.by_provider.is_empty() {
            for (provider, prov_stats) in &ai_usage.by_provider {
                println!(
                    "    {}: {} in, {} out",
                    provider.dimmed(),
                    format_number(prov_stats.tokens_in),
                    format_number(prov_stats.tokens_out)
                );
            }
        }
    }

    println!();
    Ok(())
}

fn export_stats(db: &Database, period: &str, format: &str, cwd: &str) -> Result<()> {
    let stats = db.get_command_stats(cwd)?;
    let usage_period = match period {
        "today" => UsagePeriod::Today,
        "week" => UsagePeriod::Week,
        "month" => UsagePeriod::Month,
        _ => UsagePeriod::All,
    };
    let ai_usage = db.get_ai_usage(usage_period)?;

    match format {
        "json" => {
            let output = serde_json::json!({
                "period": period,
                "commands": {
                    "total": stats.total_commands,
                    "unique": stats.unique_commands,
                    "error_rate": stats.error_rate,
                    "most_used": stats.most_used.iter()
                        .map(|(cmd, count)| serde_json::json!({"command": cmd, "count": count}))
                        .collect::<Vec<_>>()
                },
                "ai_usage": {
                    "total_tokens_in": ai_usage.total_tokens_in,
                    "total_tokens_out": ai_usage.total_tokens_out,
                    "total_cost": ai_usage.total_cost,
                    "by_provider": ai_usage.by_provider
                }
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        "csv" => {
            println!("command,count");
            for (cmd, count) in &stats.most_used {
                println!("\"{}\",{}", cmd.replace('"', "\"\""), count);
            }
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, csv", format);
        }
    }
    Ok(())
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
