use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runbook {
    pub runbook: RunbookMeta,
    pub steps: Vec<RunbookStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookMeta {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub estimated_time: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookStep {
    pub title: String,
    pub command: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub confirm: bool,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(default)]
    pub on_failure: OnFailure,
    pub fallback_command: Option<String>,
    #[serde(default)]
    pub background: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnFailure {
    #[default]
    Abort,
    Skip,
    Retry,
}

impl Runbook {
    /// Parse a runbook from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read runbook: {}", path.display()))?;
        Self::parse_toml(&contents)
    }

    /// Parse a runbook from a TOML string
    pub fn parse_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str).with_context(|| "Failed to parse runbook TOML")
    }

    /// Serialize the runbook to a TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).with_context(|| "Failed to serialize runbook")
    }

    /// Save the runbook to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = self.to_toml()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Substitute variables in a command string
    pub fn substitute_variables(
        command: &str,
        variables: &std::collections::HashMap<String, String>,
    ) -> String {
        let mut result = command.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }

    /// List all runbooks in the runbooks directory
    pub fn list_runbooks(runbooks_dir: &Path) -> Result<Vec<RunbookMeta>> {
        let mut runbooks = Vec::new();
        if !runbooks_dir.exists() {
            return Ok(runbooks);
        }
        for entry in std::fs::read_dir(runbooks_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Ok(rb) = Self::from_file(&path) {
                    runbooks.push(rb.runbook);
                }
            }
        }
        runbooks.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(runbooks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_runbook() {
        let toml_str = r#"
[runbook]
name = "test-runbook"
title = "Test Runbook"
description = "A test"
author = "test"
tags = ["test"]

[[steps]]
title = "Step 1"
command = "echo hello"
confirm = false

[[steps]]
title = "Step 2"
command = "echo {{name}}"
variables = ["name"]
"#;
        let rb = Runbook::parse_toml(toml_str).unwrap();
        assert_eq!(rb.runbook.name, "test-runbook");
        assert_eq!(rb.steps.len(), 2);
        assert!(!rb.steps[0].confirm);
        assert!(rb.steps[1].confirm); // default
    }

    #[test]
    fn test_substitute_variables() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("name".to_string(), "world".to_string());
        assert_eq!(
            Runbook::substitute_variables("echo {{name}}", &vars),
            "echo world"
        );
    }
}
