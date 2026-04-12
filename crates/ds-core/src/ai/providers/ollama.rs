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

const DEFAULT_MODEL: &str = "llama3.2";
const DEFAULT_HOST: &str = "http://localhost:11434";

pub struct OllamaProvider {
    client: Client,
    host: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(config: &AiProviderConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            host: config
                .host
                .clone()
                .unwrap_or_else(|| DEFAULT_HOST.to_string()),
            model: config
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.host)
    }

    fn build_messages(&self, request: &AiRequest) -> Vec<OllamaMessage> {
        let mut messages = Vec::new();

        if let Some(ref system) = request.system_prompt {
            messages.push(OllamaMessage {
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
            messages.push(OllamaMessage {
                role: role.to_string(),
                content: msg.content.clone(),
            });
        }

        messages
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn is_available(&self) -> bool {
        // Best-effort synchronous check: try to connect to the Ollama API
        // using a raw TCP connection to the host.  A full HTTP health-check
        // would require async; this is sufficient for a quick probe.
        use std::net::TcpStream;
        use std::time::Duration;

        let host = self
            .host
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        // Ensure we have a port (default 11434).
        let addr = if host.contains(':') {
            host.to_string()
        } else {
            format!("{host}:11434")
        };

        TcpStream::connect_timeout(
            &addr
                .parse()
                .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 11434))),
            Duration::from_millis(500),
        )
        .is_ok()
    }

    async fn complete(&self, request: &AiRequest) -> Result<AiResponse> {
        let messages = self.build_messages(request);

        let body = OllamaRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            options: request.temperature.map(|t| OllamaOptions {
                temperature: Some(t),
            }),
        };

        debug!(model = %self.model, host = %self.host, "sending Ollama completion request");

        let resp = self
            .client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .context("failed to reach Ollama API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama API error {status}: {text}"));
        }

        let result: OllamaResponse = resp
            .json()
            .await
            .context("failed to parse Ollama response")?;

        Ok(AiResponse {
            content: result.message.content,
            // Ollama may return token counts; use them if present.
            tokens_in: result.prompt_eval_count.unwrap_or(0),
            tokens_out: result.eval_count.unwrap_or(0),
            model: self.model.clone(),
            provider: "ollama".to_string(),
        })
    }

    async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let messages = self.build_messages(request);

        let body = OllamaRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            options: request.temperature.map(|t| OllamaOptions {
                temperature: Some(t),
            }),
        };

        debug!(model = %self.model, host = %self.host, "sending Ollama streaming request");

        let resp = self
            .client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .context("failed to reach Ollama API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama API error {status}: {text}"));
        }

        // Ollama streams newline-delimited JSON (not SSE).
        let byte_stream = resp.bytes_stream();

        let stream = byte_stream.filter_map(|result| match result {
            Err(e) => Some(Err(anyhow::Error::from(e))),
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                parse_ndjson_chunk(&text)
            }
        });

        Ok(Box::pin(stream))
    }
}

/// Parse a newline-delimited JSON chunk from the Ollama streaming response.
fn parse_ndjson_chunk(text: &str) -> Option<Result<StreamChunk>> {
    let mut content = String::new();
    let mut done = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(line) {
            content.push_str(&chunk.message.content);
            if chunk.done {
                done = true;
            }
        }
    }

    if content.is_empty() && !done {
        return None;
    }

    Some(Ok(StreamChunk { content, done }))
}

// ---------------------------------------------------------------------------
// Ollama API types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    message: OllamaStreamMessage,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamMessage {
    content: String,
}
