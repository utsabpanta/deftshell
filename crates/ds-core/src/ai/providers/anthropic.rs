use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::StreamExt;
use tracing::debug;

use super::resolve_api_key;
use crate::ai::gateway::{AiProvider, AiRequest, AiResponse, MessageRole, StreamChunk};
use crate::config::schema::AiProviderConfig;

const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    client: Client,
    api_key_env: String,
    model: String,
    max_tokens: u32,
}

impl AnthropicProvider {
    pub fn new(config: &AiProviderConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_default(),
            api_key_env: config
                .api_key_env
                .clone()
                .unwrap_or_else(|| "ANTHROPIC_API_KEY".to_string()),
            model: config
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens: config.max_tokens.unwrap_or(4096),
        }
    }

    fn api_key(&self) -> Result<String> {
        resolve_api_key("anthropic", &self.api_key_env, "anthropic_api_key")
    }

    fn build_body(&self, request: &AiRequest, stream: bool) -> AnthropicRequest {
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => unreachable!(),
                },
                content: m.content.clone(),
            })
            .collect();

        let system = request.system_prompt.clone().or_else(|| {
            request
                .messages
                .iter()
                .find(|m| m.role == MessageRole::System)
                .map(|m| m.content.clone())
        });

        AnthropicRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(self.max_tokens),
            system,
            messages,
            temperature: request.temperature,
            stream,
        }
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn is_available(&self) -> bool {
        self.api_key().is_ok()
    }

    async fn complete(&self, request: &AiRequest) -> Result<AiResponse> {
        let api_key = self.api_key()?;
        let body = self.build_body(request, false);

        debug!(model = %self.model, "sending Anthropic completion request");

        let resp = self
            .client
            .post(API_URL)
            .header("x-api-key", &api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to reach Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic API error {status}: {text}"));
        }

        let result: AnthropicResponse = resp
            .json()
            .await
            .context("failed to parse Anthropic response")?;

        let content = result
            .content
            .into_iter()
            .filter_map(|b| {
                if b.r#type == "text" {
                    Some(b.text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(AiResponse {
            content,
            tokens_in: result.usage.input_tokens,
            tokens_out: result.usage.output_tokens,
            model: result.model,
            provider: "anthropic".to_string(),
        })
    }

    async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let api_key = self.api_key()?;
        let body = self.build_body(request, true);

        debug!(model = %self.model, "sending Anthropic streaming request");

        let resp = self
            .client
            .post(API_URL)
            .header("x-api-key", &api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to reach Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic API error {status}: {text}"));
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

/// Parse SSE lines from an Anthropic streaming response chunk.
fn parse_sse_events(text: &str) -> Option<Result<StreamChunk>> {
    let mut content = String::new();
    let mut done = false;

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                done = true;
                continue;
            }
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

                match event_type {
                    "content_block_delta" => {
                        if let Some(delta) = event.get("delta") {
                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                content.push_str(text);
                            }
                        }
                    }
                    "message_stop" => {
                        done = true;
                    }
                    _ => {}
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
// Anthropic API types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    r#type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}
