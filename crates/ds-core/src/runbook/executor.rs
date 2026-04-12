use super::parser::{OnFailure, Runbook, RunbookStep};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub auto_confirm: bool,
    pub dry_run: bool,
    pub from_step: Option<usize>,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_index: usize,
    pub title: String,
    pub command: String,
    pub success: bool,
    pub output: String,
    pub skipped: bool,
}

pub struct RunbookExecutor;

impl RunbookExecutor {
    /// Execute a runbook with the given options
    pub fn execute(
        runbook: &Runbook,
        options: &ExecutionOptions,
        confirm_fn: &dyn Fn(&RunbookStep, &str) -> Result<bool>,
    ) -> Result<Vec<StepResult>> {
        let start_step = options.from_step.unwrap_or(0);
        let mut results = Vec::new();

        // Collect all required variables
        let variables = options.variables.clone();

        for (i, step) in runbook.steps.iter().enumerate() {
            if i < start_step {
                continue;
            }

            // Substitute variables in command
            let command = Runbook::substitute_variables(&step.command, &variables);

            // Dry run: just show what would happen
            if options.dry_run {
                results.push(StepResult {
                    step_index: i,
                    title: step.title.clone(),
                    command: command.clone(),
                    success: true,
                    output: "[dry run]".to_string(),
                    skipped: false,
                });
                continue;
            }

            // Confirm if needed
            if step.confirm && !options.auto_confirm {
                let confirmed = confirm_fn(step, &command)?;
                if !confirmed {
                    results.push(StepResult {
                        step_index: i,
                        title: step.title.clone(),
                        command: command.clone(),
                        success: true,
                        output: String::new(),
                        skipped: true,
                    });
                    continue;
                }
            }

            // Execute the command
            let result = Self::run_command(&command, step.background)?;

            if !result.success {
                match step.on_failure {
                    OnFailure::Abort => {
                        // Try fallback first
                        if let Some(ref fallback) = step.fallback_command {
                            let fallback_cmd = Runbook::substitute_variables(fallback, &variables);
                            let fallback_result = Self::run_command(&fallback_cmd, false)?;
                            results.push(StepResult {
                                step_index: i,
                                title: format!("{} (fallback)", step.title),
                                command: fallback_cmd,
                                success: fallback_result.success,
                                output: fallback_result.output,
                                skipped: false,
                            });
                            if !fallback_result.success {
                                bail!(
                                    "Step {} '{}' failed and fallback also failed",
                                    i + 1,
                                    step.title
                                );
                            }
                        } else {
                            results.push(result);
                            bail!("Step {} '{}' failed", i + 1, step.title);
                        }
                    }
                    OnFailure::Skip => {
                        results.push(StepResult {
                            step_index: i,
                            title: step.title.clone(),
                            command: command.clone(),
                            success: false,
                            output: format!("[skipped due to failure] {}", result.output),
                            skipped: true,
                        });
                    }
                    OnFailure::Retry => {
                        // Retry once
                        let retry_result = Self::run_command(&command, step.background)?;
                        results.push(StepResult {
                            step_index: i,
                            title: format!("{} (retry)", step.title),
                            command: command.clone(),
                            success: retry_result.success,
                            output: retry_result.output,
                            skipped: false,
                        });
                        if !retry_result.success {
                            bail!("Step {} '{}' failed after retry", i + 1, step.title);
                        }
                    }
                }
            } else {
                results.push(result);
            }
        }

        Ok(results)
    }

    fn run_command(command: &str, background: bool) -> Result<StepResult> {
        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        };
        let flag = if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        };

        if background {
            Command::new(shell)
                .arg(flag)
                .arg(format!("{} &", command))
                .spawn()?;

            return Ok(StepResult {
                step_index: 0,
                title: String::new(),
                command: command.to_string(),
                success: true,
                output: "[running in background]".to_string(),
                skipped: false,
            });
        }

        let output = Command::new(shell).arg(flag).arg(command).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        Ok(StepResult {
            step_index: 0,
            title: String::new(),
            command: command.to_string(),
            success: output.status.success(),
            output: combined,
            skipped: false,
        })
    }
}
