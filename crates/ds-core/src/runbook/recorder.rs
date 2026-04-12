use super::parser::{Runbook, RunbookMeta, RunbookStep};
use anyhow::Result;
use chrono::Utc;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct RecordedCommand {
    pub command: String,
    pub exit_code: i32,
    pub timestamp: chrono::DateTime<Utc>,
}

pub struct RunbookRecorder {
    recording: Arc<Mutex<bool>>,
    commands: Arc<Mutex<Vec<RecordedCommand>>>,
    name: Arc<Mutex<Option<String>>>,
}

impl RunbookRecorder {
    pub fn new() -> Self {
        Self {
            recording: Arc::new(Mutex::new(false)),
            commands: Arc::new(Mutex::new(Vec::new())),
            name: Arc::new(Mutex::new(None)),
        }
    }

    /// Start recording commands
    pub fn start(&self, name: Option<String>) {
        *self.recording.lock().unwrap() = true;
        *self.commands.lock().unwrap() = Vec::new();
        *self.name.lock().unwrap() = name;
    }

    /// Record a command during an active recording session
    pub fn record_command(&self, command: &str, exit_code: i32) {
        if !*self.recording.lock().unwrap() {
            return;
        }
        self.commands.lock().unwrap().push(RecordedCommand {
            command: command.to_string(),
            exit_code,
            timestamp: Utc::now(),
        });
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        *self.recording.lock().unwrap()
    }

    /// Stop recording and generate a runbook
    pub fn stop(&self) -> Result<Runbook> {
        *self.recording.lock().unwrap() = false;
        let commands = self.commands.lock().unwrap().clone();
        let name = self
            .name
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| format!("recorded-{}", Utc::now().format("%Y%m%d-%H%M%S")));

        let steps: Vec<RunbookStep> = commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| RunbookStep {
                title: format!("Step {}", i + 1),
                command: cmd.command.clone(),
                description: String::new(),
                confirm: true,
                variables: Vec::new(),
                on_failure: super::parser::OnFailure::Abort,
                fallback_command: None,
                background: false,
            })
            .collect();

        Ok(Runbook {
            runbook: RunbookMeta {
                name: name.clone(),
                title: name,
                description: format!("Recorded on {}", Utc::now().format("%Y-%m-%d %H:%M")),
                author: whoami(),
                version: "0.1.0".to_string(),
                tags: vec!["recorded".to_string()],
                estimated_time: None,
                requires: Vec::new(),
            },
            steps,
        })
    }

    /// Get the current recording buffer
    pub fn current_commands(&self) -> Vec<RecordedCommand> {
        self.commands.lock().unwrap().clone()
    }
}

impl Default for RunbookRecorder {
    fn default() -> Self {
        Self::new()
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
