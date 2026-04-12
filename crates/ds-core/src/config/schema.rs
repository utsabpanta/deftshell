use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeftShellConfig {
    pub general: GeneralConfig,
    pub shell: ShellConfig,
    pub prompt: PromptConfig,
    pub ai: AiConfig,
    pub safety: SafetyConfig,
    pub analytics: AnalyticsConfig,
    pub plugins: PluginsConfig,
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub telemetry: bool,
    pub update_check: bool,
    pub update_channel: UpdateChannel,
    pub log_level: LogLevel,
    pub log_file: Option<PathBuf>,
    pub data_dir: PathBuf,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".deftshell");
        Self {
            telemetry: false,
            update_check: true,
            update_channel: UpdateChannel::Stable,
            log_level: LogLevel::Warn,
            log_file: Some(data_dir.join("logs/deftshell.log")),
            data_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
    Nightly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellConfig {
    pub default: ShellType,
    pub integration_mode: IntegrationMode,
    pub vi_mode: bool,
    pub history_limit: u64,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            default: ShellType::Zsh,
            integration_mode: IntegrationMode::Full,
            vi_mode: false,
            history_limit: 50000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellType {
    Zsh,
    Bash,
    Fish,
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellType::Zsh => write!(f, "zsh"),
            ShellType::Bash => write!(f, "bash"),
            ShellType::Fish => write!(f, "fish"),
        }
    }
}

impl std::str::FromStr for ShellType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zsh" => Ok(ShellType::Zsh),
            "bash" => Ok(ShellType::Bash),
            "fish" => Ok(ShellType::Fish),
            _ => anyhow::bail!("Unknown shell type: {s}. Supported: zsh, bash, fish"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationMode {
    Full,
    PromptOnly,
    Passive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptConfig {
    pub theme: PromptTheme,
    pub show_git: bool,
    pub show_stack: bool,
    pub show_env: bool,
    pub show_services: bool,
    pub show_ai_status: bool,
    pub show_execution_time: bool,
    pub execution_time_threshold_ms: u64,
    pub show_kubernetes: bool,
    pub show_aws_profile: bool,
    pub transient_prompt: bool,
    pub right_prompt: bool,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            theme: PromptTheme::Default,
            show_git: true,
            show_stack: true,
            show_env: true,
            show_services: true,
            show_ai_status: true,
            show_execution_time: true,
            execution_time_threshold_ms: 2000,
            show_kubernetes: true,
            show_aws_profile: true,
            transient_prompt: true,
            right_prompt: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptTheme {
    Default,
    Minimal,
    Powerline,
    Pure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub default_provider: String,
    pub fallback_provider: Option<String>,
    pub privacy_mode_provider: Option<String>,
    pub privacy_mode: bool,
    pub providers: HashMap<String, AiProviderConfig>,
    pub context: AiContextConfig,
    pub limits: AiLimitsConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_provider: "ollama".to_string(),
            fallback_provider: None,
            privacy_mode_provider: Some("ollama".to_string()),
            privacy_mode: false,
            providers: HashMap::new(),
            context: AiContextConfig::default(),
            limits: AiLimitsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiProviderConfig {
    pub enabled: bool,
    pub api_key_env: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub host: Option<String>,
    pub aws_profile: Option<String>,
    pub region: Option<String>,
    pub model_id: Option<String>,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key_env: None,
            model: None,
            max_tokens: Some(4096),
            host: None,
            aws_profile: None,
            region: None,
            model_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiContextConfig {
    pub include_files: Vec<String>,
    pub exclude_files: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl Default for AiContextConfig {
    fn default() -> Self {
        Self {
            include_files: Vec::new(),
            exclude_files: vec![
                ".env*".to_string(),
                "secrets/".to_string(),
                "*.key".to_string(),
                "*.pem".to_string(),
            ],
            exclude_patterns: vec![
                "password".to_string(),
                "api_key".to_string(),
                "secret".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiLimitsConfig {
    pub daily_token_limit: u64,
    pub per_request_token_limit: u32,
    pub warn_at_percentage: u8,
}

impl Default for AiLimitsConfig {
    fn default() -> Self {
        Self {
            daily_token_limit: 100_000,
            per_request_token_limit: 8000,
            warn_at_percentage: 80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    pub enabled: bool,
    pub level: SafetyLevel,
    pub confirm_timeout_seconds: u32,
    pub log_intercepted: bool,
    pub custom_rules: Vec<CustomSafetyRule>,
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: SafetyLevel::Standard,
            confirm_timeout_seconds: 30,
            log_intercepted: true,
            custom_rules: Vec::new(),
            allowlist: Vec::new(),
            denylist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SafetyLevel {
    Strict,
    Standard,
    Relaxed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSafetyRule {
    pub pattern: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalyticsConfig {
    pub enabled: bool,
    pub retention_days: u32,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 365,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginsConfig {
    pub auto_update: bool,
    pub registry: String,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            auto_update: true,
            registry: "npm".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    pub primary: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    pub info: String,
    pub muted: String,
    pub ai_response: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            primary: "#7C3AED".to_string(),
            success: "#10B981".to_string(),
            warning: "#F59E0B".to_string(),
            danger: "#EF4444".to_string(),
            info: "#3B82F6".to_string(),
            muted: "#6B7280".to_string(),
            ai_response: "#8B5CF6".to_string(),
        }
    }
}

/// Project-level configuration (.deftshell.toml)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    pub stack: StackOverrides,
    pub scripts: HashMap<String, String>,
    pub ai: ProjectAiConfig,
    pub safety: ProjectSafetyConfig,
    pub aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectInfo {
    pub name: Option<String>,
    pub team: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StackOverrides {
    pub primary_language: Option<String>,
    pub runtime: Option<String>,
    pub framework: Option<String>,
    pub test_runner: Option<String>,
    pub linter: Option<String>,
    pub formatter: Option<String>,
    pub bundler: Option<String>,
    pub package_manager: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectAiConfig {
    pub context: AiContextConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSafetyConfig {
    pub custom_rules: Vec<CustomSafetyRule>,
}
