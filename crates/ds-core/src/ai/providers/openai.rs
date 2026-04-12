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

const DEFAULT_MODEL: &str = "gpt-4o";
const API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiProvider {
    client: Client,
    api_key_env: String,
    model: String,
    max_tokens: u32,
}

impl OpenAiProvider {
    pub fn new(config: &AiProviderConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_default(),
            api_key_env: config
                .api_key_env
                .clone()
                .unwrap_or_else(|| "OPENAI_API_KEY".to_string()),
            model: config
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens: config.max_tokens.unwrap_or(4096),
        }
    }

    fn api_key(&self) -> Result<String> {
        resolve_api_key("openai", &self.api_key_env, "openai_api_key")
    }

    fn build_messages(&self, request: &AiRequest) -> Vec<OpenAiMessage> {
        let mut messages = Vec::new();

        // Add system prompt if present.
        if let Some(ref system) = request.system_prompt {
            messages.push(OpenAiMessage {
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
            messages.push(OpenAiMessage {
                role: role.to_string(),
                content: msg.content.clone(),
            });
        }

        messages
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn is_available(&self) -> bool {
        self.api_key().is_ok()
    }

    async fn complete(&self, request: &AiRequest) -> Result<AiResponse> {
        let api_key = self.api_key()?;
        let messages = self.build_messages(request);

        let body = OpenAiRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens.unwrap_or(self.max_tokens)),
            temperature: request.temperature,
            stream: false,
        };

        debug!(model = %self.model, "sending OpenAI completion request");

        let resp = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to reach OpenAI API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error {status}: {text}"));
        }

        let result: OpenAiResponse = resp
            .json()
            .await
            .context("failed to parse OpenAI response")?;

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
            provider: "openai".to_string(),
        })
    }

    async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let api_key = self.api_key()?;
        let messages = self.build_messages(request);

        let body = OpenAiRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens.unwrap_or(self.max_tokens)),
            temperature: request.temperature,
            stream: true,
        };

        debug!(model = %self.model, "sending OpenAI streaming request");

        let resp = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to reach OpenAI API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error {status}: {text}"));
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

/// Parse SSE lines from an OpenAI streaming response chunk.
fn parse_sse_events(text: &str) -> Option<Result<StreamChunk>> {
    let mut content = String::new();
    let mut done = false;

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                done = true;
                continue;
            }
            if let Ok(event) = serde_json::from_str::<OpenAiStreamChunk>(data) {
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
// OpenAI API types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    model: String,
    usage: OpenAiUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageResponse,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}
