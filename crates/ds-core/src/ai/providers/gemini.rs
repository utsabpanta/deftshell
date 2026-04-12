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

const DEFAULT_MODEL: &str = "gemini-pro";
const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiProvider {
    client: Client,
    api_key_env: String,
    model: String,
    max_tokens: u32,
}

impl GeminiProvider {
    pub fn new(config: &AiProviderConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_default(),
            api_key_env: config
                .api_key_env
                .clone()
                .unwrap_or_else(|| "GEMINI_API_KEY".to_string()),
            model: config
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens: config.max_tokens.unwrap_or(4096),
        }
    }

    fn api_key(&self) -> Result<String> {
        resolve_api_key("gemini", &self.api_key_env, "gemini_api_key")
    }

    fn generate_url(&self) -> String {
        format!("{API_BASE}/{}:generateContent", self.model)
    }

    fn stream_url(&self) -> String {
        format!("{API_BASE}/{}:streamGenerateContent", self.model)
    }

    fn build_body(&self, request: &AiRequest) -> GeminiRequest {
        let mut contents = Vec::new();

        // Map chat messages to Gemini's content format.
        for msg in &request.messages {
            let role = match msg.role {
                MessageRole::User | MessageRole::System => "user",
                MessageRole::Assistant => "model",
            };
            contents.push(GeminiContent {
                role: role.to_string(),
                parts: vec![GeminiPart {
                    text: msg.content.clone(),
                }],
            });
        }

        let system_instruction = request.system_prompt.as_ref().map(|s| GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart { text: s.clone() }],
        });

        GeminiRequest {
            contents,
            system_instruction,
            generation_config: Some(GeminiGenerationConfig {
                max_output_tokens: Some(request.max_tokens.unwrap_or(self.max_tokens)),
                temperature: request.temperature,
            }),
        }
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn is_available(&self) -> bool {
        self.api_key().is_ok()
    }

    async fn complete(&self, request: &AiRequest) -> Result<AiResponse> {
        let api_key = self.api_key()?;
        let body = self.build_body(request);
        let url = self.generate_url();

        debug!(model = %self.model, "sending Gemini completion request");

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &api_key)
            .json(&body)
            .send()
            .await
            .context("failed to reach Gemini API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini API error {status}: {text}"));
        }

        let result: GeminiResponse = resp
            .json()
            .await
            .context("failed to parse Gemini response")?;

        let content = result
            .candidates
            .into_iter()
            .flat_map(|c| c.content.parts)
            .map(|p| p.text)
            .collect::<Vec<_>>()
            .join("");

        let (tokens_in, tokens_out) = result
            .usage_metadata
            .map(|u| (u.prompt_token_count, u.candidates_token_count))
            .unwrap_or((0, 0));

        Ok(AiResponse {
            content,
            tokens_in,
            tokens_out,
            model: self.model.clone(),
            provider: "gemini".to_string(),
        })
    }

    async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let api_key = self.api_key()?;
        let body = self.build_body(request);
        let url = format!("{}?alt=sse", self.stream_url());

        debug!(model = %self.model, "sending Gemini streaming request");

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &api_key)
            .json(&body)
            .send()
            .await
            .context("failed to reach Gemini API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini API error {status}: {text}"));
        }

        let byte_stream = resp.bytes_stream();

        let stream = byte_stream.filter_map(|result| match result {
            Err(e) => Some(Err(anyhow::Error::from(e))),
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                parse_gemini_sse(&text)
            }
        });

        Ok(Box::pin(stream))
    }
}

/// Parse SSE events from a Gemini streaming response chunk.
fn parse_gemini_sse(text: &str) -> Option<Result<StreamChunk>> {
    let mut content = String::new();
    let mut done = false;

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(chunk) = serde_json::from_str::<GeminiResponse>(data) {
                for candidate in &chunk.candidates {
                    for part in &candidate.content.parts {
                        content.push_str(&part.text);
                    }
                    // Gemini signals completion via finish_reason.
                    if candidate.finish_reason.is_some() {
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
// Gemini API types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    prompt_token_count: u32,
    candidates_token_count: u32,
}
