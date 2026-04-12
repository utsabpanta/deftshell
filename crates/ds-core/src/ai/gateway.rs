use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tracing::{info, warn};

use crate::config::schema::{AiConfig, AiProviderConfig};

use super::providers::{
    anthropic::AnthropicProvider, bedrock::BedrockProvider, copilot::CopilotProvider,
    gemini::GeminiProvider, ollama::OllamaProvider, openai::OpenAiProvider,
};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AiRequest {
    pub system_prompt: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct AiResponse {
    pub content: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Human-readable provider name (e.g. "anthropic", "openai").
    fn name(&self) -> &str;

    /// Returns `true` when the provider is configured and reachable.
    fn is_available(&self) -> bool;

    /// Send a non-streaming completion request.
    async fn complete(&self, request: &AiRequest) -> Result<AiResponse>;

    /// Send a streaming completion request.
    async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;
}

// ---------------------------------------------------------------------------
// AI Gateway
// ---------------------------------------------------------------------------

pub struct AiGateway {
    providers: Vec<Box<dyn AiProvider>>,
    default_provider: String,
    fallback_provider: Option<String>,
    privacy_mode: bool,
    privacy_provider: Option<String>,
}

impl AiGateway {
    /// All known built-in provider names.
    const KNOWN_PROVIDERS: &'static [&'static str] = &[
        "anthropic",
        "openai",
        "ollama",
        "copilot",
        "bedrock",
        "gemini",
    ];

    /// Create a new gateway from the given configuration.
    ///
    /// All known providers are auto-registered with default settings.  If a
    /// provider is explicitly listed in `config.providers` its settings are
    /// used; if that entry has `enabled = false` the provider is skipped.
    /// Providers *not* mentioned in config are registered with defaults so
    /// that users only need to set `default_provider` and have credentials —
    /// no manual `[ai.providers.X]` block required.
    pub fn new(config: &AiConfig) -> Self {
        let mut providers: Vec<Box<dyn AiProvider>> = Vec::new();

        for &name in Self::KNOWN_PROVIDERS {
            let provider_config = config.providers.get(name);

            // If explicitly disabled in config, skip.
            if let Some(cfg) = provider_config {
                if !cfg.enabled && config.providers.contains_key(name) {
                    // Only skip if the user *explicitly* set enabled = false.
                    // We check this by seeing if a config entry exists at all.
                    continue;
                }
            }

            let effective_config = provider_config
                .cloned()
                .unwrap_or_else(AiProviderConfig::default);

            if let Some(provider) = Self::create_provider(name, &effective_config) {
                info!(provider = name, "registered AI provider");
                providers.push(provider);
            }
        }

        // Also register any custom/unknown providers from config that are enabled.
        for (name, provider_config) in &config.providers {
            if Self::KNOWN_PROVIDERS.contains(&name.as_str()) {
                continue; // Already handled above.
            }
            if !provider_config.enabled {
                continue;
            }
            if let Some(provider) = Self::create_provider(name, provider_config) {
                info!(provider = name, "registered custom AI provider");
                providers.push(provider);
            } else {
                warn!(provider = name, "unknown AI provider -- skipped");
            }
        }

        Self {
            providers,
            default_provider: config.default_provider.clone(),
            fallback_provider: config.fallback_provider.clone(),
            privacy_mode: config.privacy_mode,
            privacy_provider: config.privacy_mode_provider.clone(),
        }
    }

    // -- public interface ---------------------------------------------------

    /// Run a (non-streaming) completion against the default provider, falling
    /// back to the explicitly-configured fallback provider on error.
    pub async fn complete(&self, request: &AiRequest) -> Result<AiResponse> {
        let provider_name = self.active_provider_name();

        if let Some(provider) = self.get_provider(&provider_name) {
            match provider.complete(request).await {
                Ok(response) => return Ok(response),
                Err(primary_error) => {
                    if let Some(ref fallback_name) = self.fallback_provider {
                        warn!(
                            provider = provider_name,
                            fallback = fallback_name,
                            error = %primary_error,
                            "primary provider failed, trying fallback"
                        );
                        if let Some(fallback) = self.get_provider(fallback_name) {
                            return fallback.complete(request).await.with_context(|| {
                                format!(
                                    "fallback provider `{fallback_name}` also failed \
                                     (primary `{provider_name}` error: {primary_error})"
                                )
                            });
                        }
                    }
                    return Err(
                        primary_error.context(format!("AI provider `{provider_name}` failed"))
                    );
                }
            }
        }

        Err(anyhow!(
            "AI provider `{}` is not registered. \
             Run `ds auth status` to see available providers.",
            provider_name
        ))
    }

    /// Run a streaming completion against the default provider, falling back
    /// to the explicitly-configured fallback provider on error.
    pub async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let provider_name = self.active_provider_name();

        if let Some(provider) = self.get_provider(&provider_name) {
            match provider.stream(request).await {
                Ok(stream) => return Ok(stream),
                Err(primary_error) => {
                    // Only try fallback if one is explicitly configured.
                    if let Some(ref fallback_name) = self.fallback_provider {
                        warn!(
                            provider = provider_name,
                            fallback = fallback_name,
                            error = %primary_error,
                            "primary provider failed, trying fallback"
                        );
                        if let Some(fallback) = self.get_provider(fallback_name) {
                            return fallback.stream(request).await.with_context(|| {
                                format!(
                                    "fallback provider `{fallback_name}` also failed \
                                     (primary `{provider_name}` error: {primary_error})"
                                )
                            });
                        }
                    }
                    // No fallback — return the primary error directly.
                    return Err(
                        primary_error.context(format!("AI provider `{provider_name}` failed"))
                    );
                }
            }
        }

        Err(anyhow!(
            "AI provider `{}` is not registered. \
             Run `ds auth status` to see available providers.",
            provider_name
        ))
    }

    /// Override the default provider for this gateway instance.
    pub fn set_provider(&mut self, provider: &str) {
        self.default_provider = provider.to_string();
    }

    /// Toggle privacy mode.  When enabled the gateway routes requests through
    /// the privacy-mode provider (typically a local model like Ollama).
    pub fn set_privacy_mode(&mut self, enabled: bool) {
        self.privacy_mode = enabled;
        info!(privacy_mode = enabled, "privacy mode toggled");
    }

    /// Look up a registered provider by name.
    pub fn get_provider(&self, name: &str) -> Option<&dyn AiProvider> {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// Return a list of `(name, is_available)` for every registered provider.
    pub fn list_providers(&self) -> Vec<(&str, bool)> {
        self.providers
            .iter()
            .map(|p| (p.name(), p.is_available()))
            .collect()
    }

    // -- helpers ------------------------------------------------------------

    /// Determine which provider name should handle the current request taking
    /// privacy mode into account.
    fn active_provider_name(&self) -> String {
        if self.privacy_mode {
            self.privacy_provider
                .clone()
                .unwrap_or_else(|| self.default_provider.clone())
        } else {
            self.default_provider.clone()
        }
    }

    /// Instantiate a provider from its configuration.
    fn create_provider(name: &str, config: &AiProviderConfig) -> Option<Box<dyn AiProvider>> {
        match name {
            "anthropic" => Some(Box::new(AnthropicProvider::new(config))),
            "openai" => Some(Box::new(OpenAiProvider::new(config))),
            "ollama" => Some(Box::new(OllamaProvider::new(config))),
            "copilot" => Some(Box::new(CopilotProvider::new(config))),
            "bedrock" => Some(Box::new(BedrockProvider::new(config))),
            "gemini" => Some(Box::new(GeminiProvider::new(config))),
            _ => None,
        }
    }
}
