use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::schema::{DeftShellConfig, ProjectConfig};

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load the full configuration with hierarchy:
    /// defaults -> user config -> project config -> env vars
    pub fn load(project_dir: Option<&Path>) -> Result<DeftShellConfig> {
        let mut config = DeftShellConfig::default();

        // Layer 2: User config (~/.deftshell/config.toml)
        if let Some(user_config_path) = Self::user_config_path() {
            if user_config_path.exists() {
                let contents = std::fs::read_to_string(&user_config_path)
                    .with_context(|| format!("Failed to read {}", user_config_path.display()))?;
                let user_config: DeftShellConfig = toml::from_str(&contents)
                    .with_context(|| format!("Failed to parse {}", user_config_path.display()))?;
                config = Self::merge_config(config, user_config);
            }
        }

        // Layer 3: Project config (.deftshell.toml)
        if let Some(dir) = project_dir {
            let project_config_path = dir.join(".deftshell.toml");
            if project_config_path.exists() {
                let contents = std::fs::read_to_string(&project_config_path)
                    .with_context(|| format!("Failed to read {}", project_config_path.display()))?;
                let project_config: ProjectConfig =
                    toml::from_str(&contents).with_context(|| {
                        format!("Failed to parse {}", project_config_path.display())
                    })?;
                config = Self::apply_project_config(config, project_config);
            }
        }

        // Layer 4: Environment variables
        config = Self::apply_env_vars(config);

        Ok(config)
    }

    /// Load just the project-level config
    pub fn load_project_config(project_dir: &Path) -> Result<Option<ProjectConfig>> {
        let config_path = project_dir.join(".deftshell.toml");
        if !config_path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let config: ProjectConfig = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?;
        Ok(Some(config))
    }

    pub fn user_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".deftshell/config.toml"))
    }

    pub fn data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".deftshell")
    }

    pub fn ensure_data_dir() -> Result<PathBuf> {
        let dir = Self::data_dir();
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create data directory: {}", dir.display()))?;
        std::fs::create_dir_all(dir.join("logs")).ok();
        std::fs::create_dir_all(dir.join("runbooks")).ok();
        std::fs::create_dir_all(dir.join("plugins")).ok();
        Ok(dir)
    }

    pub fn db_path() -> PathBuf {
        Self::data_dir().join("db.sqlite")
    }

    #[allow(unused_variables)]
    fn merge_config(base: DeftShellConfig, overlay: DeftShellConfig) -> DeftShellConfig {
        // The overlay replaces base values (TOML deserialization handles defaults)
        DeftShellConfig {
            general: overlay.general,
            shell: overlay.shell,
            prompt: overlay.prompt,
            ai: overlay.ai,
            safety: overlay.safety,
            analytics: overlay.analytics,
            plugins: overlay.plugins,
            theme: overlay.theme,
        }
    }

    fn apply_project_config(
        mut config: DeftShellConfig,
        project: ProjectConfig,
    ) -> DeftShellConfig {
        // Merge AI context exclusions from project config
        if !project.ai.context.exclude_files.is_empty() {
            config
                .ai
                .context
                .exclude_files
                .extend(project.ai.context.exclude_files);
        }
        if !project.ai.context.exclude_patterns.is_empty() {
            config
                .ai
                .context
                .exclude_patterns
                .extend(project.ai.context.exclude_patterns);
        }
        if !project.ai.context.include_files.is_empty() {
            config
                .ai
                .context
                .include_files
                .extend(project.ai.context.include_files);
        }
        // Merge safety custom rules
        if !project.safety.custom_rules.is_empty() {
            config
                .safety
                .custom_rules
                .extend(project.safety.custom_rules);
        }
        config
    }

    fn apply_env_vars(mut config: DeftShellConfig) -> DeftShellConfig {
        if let Ok(provider) = std::env::var("DS_AI_PROVIDER") {
            config.ai.default_provider = provider;
        }
        if let Ok(level) = std::env::var("DS_SAFETY_LEVEL") {
            match level.to_lowercase().as_str() {
                "strict" => config.safety.level = super::schema::SafetyLevel::Strict,
                "standard" => config.safety.level = super::schema::SafetyLevel::Standard,
                "relaxed" => config.safety.level = super::schema::SafetyLevel::Relaxed,
                _ => {}
            }
        }
        if let Ok(val) = std::env::var("DS_LOG_LEVEL") {
            match val.to_lowercase().as_str() {
                "trace" => config.general.log_level = super::schema::LogLevel::Trace,
                "debug" => config.general.log_level = super::schema::LogLevel::Debug,
                "info" => config.general.log_level = super::schema::LogLevel::Info,
                "warn" => config.general.log_level = super::schema::LogLevel::Warn,
                "error" => config.general.log_level = super::schema::LogLevel::Error,
                _ => {}
            }
        }
        if let Ok(val) = std::env::var("DS_PRIVACY_MODE") {
            match val.to_lowercase().as_str() {
                "0" | "false" | "off" | "no" | "" => config.ai.privacy_mode = false,
                _ => config.ai.privacy_mode = true,
            }
        }
        config
    }

    /// Write config to the user config file
    pub fn save_user_config(config: &DeftShellConfig) -> Result<()> {
        let path = Self::user_config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(config)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DeftShellConfig::default();
        assert!(config.safety.enabled);
        assert_eq!(config.ai.default_provider, "ollama");
    }

    #[test]
    fn test_load_nonexistent_project() {
        let config = ConfigLoader::load(Some(Path::new("/nonexistent/path")));
        assert!(config.is_ok());
    }

    #[test]
    fn test_parse_project_config() {
        let toml_str = r#"
[project]
name = "test-app"
team = "platform"

[scripts]
dev = "npm run dev"
test = "npm test"

[aliases]
dev = "npm run dev"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project.name, Some("test-app".to_string()));
        assert_eq!(config.scripts.get("dev"), Some(&"npm run dev".to_string()));
    }
}
