use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::StreamExt;
use tracing::debug;

use crate::ai::gateway::{AiProvider, AiRequest, AiResponse, MessageRole, StreamChunk};
use crate::config::schema::AiProviderConfig;

const DEFAULT_MODEL: &str = "gpt-4o";
/// GitHub Models API — officially supported, works with any `gh auth token`.
const API_URL: &str = "https://models.inference.ai.azure.com/chat/completions";
const HOSTS_JSON_RELATIVE: &str = ".config/github-copilot/hosts.json";
const APPS_JSON_RELATIVE: &str = ".config/github-copilot/apps.json";

/// GitHub Copilot AI provider.
///
/// Uses the GitHub Models API (OpenAI-compatible) which works with any
/// authenticated GitHub token.
///
/// Authentication sources (checked in order):
///   1. `~/.config/github-copilot/hosts.json` (VS Code Copilot extension)
///   2. `~/.config/github-copilot/apps.json` (JetBrains Copilot plugin)
///   3. `gh auth token` (GitHub CLI)
///   4. `GITHUB_TOKEN` env var
pub struct CopilotProvider {
    client: Client,
    model: String,
    max_tokens: u32,
}

impl CopilotProvider {
    pub fn new(config: &AiProviderConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_default(),
            model: config
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens: config.max_tokens.unwrap_or(4096),
        }
    }

    /// Resolve a GitHub token from available sources.
    fn resolve_token(&self) -> Result<String> {
        // 1. Try hosts.json (VS Code Copilot extension).
        if let Some(token) = Self::read_copilot_config_token() {
            return Ok(token);
        }

        // 2. Try `gh auth token` (GitHub CLI).
        if let Some(token) = Self::read_gh_cli_token() {
            return Ok(token);
        }

        // 3. Try GITHUB_TOKEN env var.
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                return Ok(token);
            }
        }

        Err(anyhow!(
            "GitHub Copilot: no GitHub token found.\n\n\
             Set it up with one of:\n  \
             - ds auth copilot        (recommended)\n  \
             - gh auth login          (GitHub CLI)\n  \
             - export GITHUB_TOKEN=<token>"
        ))
    }

    /// Read an OAuth token from Copilot config files (hosts.json or apps.json).
    fn read_copilot_config_token() -> Option<String> {
        let home = dirs::home_dir()?;

        // Try hosts.json first, then apps.json.
        for relative_path in &[HOSTS_JSON_RELATIVE, APPS_JSON_RELATIVE] {
            let path = home.join(relative_path);
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(hosts) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(obj) = hosts.as_object() {
                        for (_host, entry) in obj {
                            if let Some(token) = entry.get("oauth_token").and_then(|v| v.as_str()) {
                                if !token.is_empty() {
                                    return Some(token.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Get a token from the GitHub CLI (`gh auth token`).
    fn read_gh_cli_token() -> Option<String> {
        let output = std::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token.is_empty() {
            None
        } else {
            Some(token)
        }
    }

    fn build_messages(&self, request: &AiRequest) -> Vec<CopilotMessage> {
        let mut messages = Vec::new();

        if let Some(ref system) = request.system_prompt {
            messages.push(CopilotMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        for msg in &request.messages {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };
            messages.push(CopilotMessage {
                role: role.to_string(),
                content: msg.content.clone(),
            });
        }

        messages
    }
}

#[async_trait]
impl AiProvider for CopilotProvider {
    fn name(&self) -> &str {
        "copilot"
    }

    fn is_available(&self) -> bool {
        Self::read_copilot_config_token().is_some()
            || Self::read_gh_cli_token().is_some()
            || std::env::var("GITHUB_TOKEN").is_ok()
    }

    async fn complete(&self, request: &AiRequest) -> Result<AiResponse> {
        let token = self.resolve_token()?;
        let messages = self.build_messages(request);

        let body = CopilotRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens.unwrap_or(self.max_tokens)),
            temperature: request.temperature,
            stream: false,
        };

        debug!(model = %self.model, "sending GitHub Models completion request");

        let resp = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to reach GitHub Models API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub Models API error {status}: {text}"));
        }

        let result: CopilotResponse = resp
            .json()
            .await
            .context("failed to parse GitHub Models response")?;

        let content = result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(AiResponse {
            content,
            tokens_in: result.usage.prompt_tokens,
            tokens_out: result.usage.completion_tokens,
            model: result.model,
            provider: "copilot".to_string(),
        })
    }

    async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let token = self.resolve_token()?;
        let messages = self.build_messages(request);

        let body = CopilotRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens.unwrap_or(self.max_tokens)),
            temperature: request.temperature,
            stream: true,
        };

        debug!(model = %self.model, "sending GitHub Models streaming request");

        let resp = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to reach GitHub Models API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub Models API error {status}: {text}"));
        }

        let byte_stream = resp.bytes_stream();

        let stream = byte_stream.filter_map(|result| match result {
            Err(e) => Some(Err(anyhow::Error::from(e))),
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                parse_sse_events(&text)
            }
        });

        Ok(Box::pin(stream))
    }
}

/// Parse SSE lines from a streaming response chunk.
fn parse_sse_events(text: &str) -> Option<Result<StreamChunk>> {
    let mut content = String::new();
    let mut done = false;

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                done = true;
                continue;
            }
            if let Ok(event) = serde_json::from_str::<CopilotStreamChunk>(data) {
                if let Some(choice) = event.choices.first() {
                    if let Some(ref c) = choice.delta.content {
                        content.push_str(c);
                    }
                    if choice.finish_reason.is_some() {
                        done = true;
                    }
                }
            }
        }
    }

    if content.is_empty() && !done {
        return None;
    }

    Some(Ok(StreamChunk { content, done }))
}

// ---------------------------------------------------------------------------
// API types (OpenAI-compatible)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct CopilotRequest {
    model: String,
    messages: Vec<CopilotMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct CopilotMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct CopilotResponse {
    choices: Vec<CopilotChoice>,
    model: String,
    usage: CopilotUsage,
}

#[derive(Debug, Deserialize)]
struct CopilotChoice {
    message: CopilotMessageResponse,
}

#[derive(Debug, Deserialize)]
struct CopilotMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct CopilotUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct CopilotStreamChunk {
    choices: Vec<CopilotStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct CopilotStreamChoice {
    delta: CopilotDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CopilotDelta {
    content: Option<String>,
}
