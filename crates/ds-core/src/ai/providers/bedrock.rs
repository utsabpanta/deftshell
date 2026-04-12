use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use ring::{digest, hmac};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::StreamExt;
use tracing::debug;

use crate::ai::gateway::{AiProvider, AiRequest, AiResponse, MessageRole, StreamChunk};
use crate::config::schema::AiProviderConfig;

const DEFAULT_MODEL_ID: &str = "anthropic.claude-3-sonnet-20240229-v1:0";
const DEFAULT_REGION: &str = "us-east-1";
const ANTHROPIC_BEDROCK_VERSION: &str = "bedrock-2023-05-31";
const SERVICE: &str = "bedrock";

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

pub struct BedrockProvider {
    client: Client,
    region: String,
    model_id: String,
    profile: Option<String>,
    max_tokens: u32,
}

impl BedrockProvider {
    pub fn new(config: &AiProviderConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap_or_default(),
            region: config
                .region
                .clone()
                .unwrap_or_else(|| DEFAULT_REGION.to_string()),
            model_id: config
                .model_id
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL_ID.to_string()),
            profile: config.aws_profile.clone(),
            max_tokens: config.max_tokens.unwrap_or(4096),
        }
    }

    /// Load AWS credentials from environment variables or `~/.aws/credentials`.
    fn load_credentials(&self) -> Result<AwsCredentials> {
        // 1. Try environment variables first.
        if let (Ok(access_key), Ok(secret_key)) = (
            std::env::var("AWS_ACCESS_KEY_ID"),
            std::env::var("AWS_SECRET_ACCESS_KEY"),
        ) {
            let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
            return Ok(AwsCredentials {
                access_key_id: access_key,
                secret_access_key: secret_key,
                session_token,
            });
        }

        // 2. Try the shared credentials file (~/.aws/credentials).
        let credentials_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("cannot determine home directory"))?
            .join(".aws")
            .join("credentials");

        if !credentials_path.exists() {
            return Err(anyhow!(
                "AWS credentials not found. Set AWS_ACCESS_KEY_ID and \
                 AWS_SECRET_ACCESS_KEY environment variables, or configure \
                 ~/.aws/credentials"
            ));
        }

        let contents = std::fs::read_to_string(&credentials_path)
            .with_context(|| format!("failed to read {}", credentials_path.display()))?;

        let env_profile = std::env::var("AWS_PROFILE").ok();
        let profile_name = self
            .profile
            .as_deref()
            .or(env_profile.as_deref())
            .unwrap_or("default");

        parse_credentials_file(&contents, profile_name)
    }

    /// Construct the Bedrock InvokeModel endpoint URL.
    fn endpoint_url(&self) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/invoke",
            self.region,
            // Model IDs contain characters like ':' that need URL-encoding.
            urlencoded(&self.model_id),
        )
    }

    /// Construct the Bedrock InvokeModelWithResponseStream endpoint URL.
    fn stream_endpoint_url(&self) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/invoke-with-response-stream",
            self.region,
            urlencoded(&self.model_id),
        )
    }

    /// Build the request body using the Anthropic Messages format
    /// (the standard for Claude models on Bedrock).
    fn build_body(&self, request: &AiRequest) -> BedrockRequestBody {
        let messages: Vec<BedrockMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| BedrockMessage {
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

        BedrockRequestBody {
            anthropic_version: ANTHROPIC_BEDROCK_VERSION.to_string(),
            max_tokens: request.max_tokens.unwrap_or(self.max_tokens),
            system,
            messages,
            temperature: request.temperature,
        }
    }

    /// Sign and send a request to the Bedrock API.
    async fn send_signed_request(&self, url: &str, body: &[u8]) -> Result<reqwest::Response> {
        let credentials = self.load_credentials()?;
        let host = format!("bedrock-runtime.{}.amazonaws.com", self.region);
        let now = chrono::Utc::now();

        let signed_headers = sign_request_v4(
            "POST",
            url,
            &host,
            body,
            &self.region,
            SERVICE,
            &credentials,
            now,
        )?;

        let mut builder = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Host", &host);

        for (key, value) in &signed_headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        let resp = builder
            .body(body.to_vec())
            .send()
            .await
            .context("failed to reach AWS Bedrock API")?;

        Ok(resp)
    }
}

#[async_trait]
impl AiProvider for BedrockProvider {
    fn name(&self) -> &str {
        "bedrock"
    }

    fn is_available(&self) -> bool {
        self.load_credentials().is_ok()
    }

    async fn complete(&self, request: &AiRequest) -> Result<AiResponse> {
        let body = self.build_body(request);
        let body_bytes = serde_json::to_vec(&body).context("failed to serialize request body")?;
        let url = self.endpoint_url();

        debug!(model_id = %self.model_id, region = %self.region, "sending Bedrock completion request");

        let resp = self.send_signed_request(&url, &body_bytes).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Bedrock API error {status}: {text}"));
        }

        let result: BedrockResponse = resp
            .json()
            .await
            .context("failed to parse Bedrock response")?;

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
            model: result.model.unwrap_or_else(|| self.model_id.clone()),
            provider: "bedrock".to_string(),
        })
    }

    async fn stream(
        &self,
        request: &AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let body = self.build_body(request);
        let body_bytes = serde_json::to_vec(&body).context("failed to serialize request body")?;
        let url = self.stream_endpoint_url();

        debug!(model_id = %self.model_id, region = %self.region, "sending Bedrock streaming request");

        let resp = self.send_signed_request(&url, &body_bytes).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Bedrock API error {status}: {text}"));
        }

        let byte_stream = resp.bytes_stream();

        // Bedrock streaming uses AWS event-stream binary framing. Each frame
        // wraps a JSON payload that uses the same Anthropic SSE event types
        // (`content_block_delta`, `message_stop`, etc.).  We accumulate raw
        // bytes and attempt to extract JSON objects delimited by `{` ... `}`.
        let stream = byte_stream.filter_map(|result| match result {
            Err(e) => Some(Err(anyhow::Error::from(e))),
            Ok(bytes) => parse_bedrock_stream_chunk(&bytes),
        });

        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Bedrock event-stream parsing
// ---------------------------------------------------------------------------

/// Parse a chunk from Bedrock's response stream.
///
/// Bedrock wraps Anthropic-format JSON events inside AWS binary event-stream
/// frames.  Each frame has headers followed by a JSON payload.  We scan for
/// JSON objects within the raw bytes and extract text deltas.
fn parse_bedrock_stream_chunk(bytes: &[u8]) -> Option<Result<StreamChunk>> {
    let mut content = String::new();
    let mut done = false;

    // The event-stream binary format encodes each event as:
    //   [total-length:4][headers-length:4][prelude-crc:4][headers][payload][message-crc:4]
    //
    // We iterate through the bytes trying to extract payloads. The payload is
    // JSON text.  Rather than fully implementing the event-stream spec we look
    // for JSON blocks that match the expected Anthropic event structure.  This
    // is pragmatic and handles the common case well.

    let text = String::from_utf8_lossy(bytes);

    // Try to find JSON objects within the binary frame data.  Bedrock wraps
    // each event with an outer `{"bytes":"<base64>"}` envelope OR, for the
    // invoke-with-response-stream API, it uses the event-stream binary
    // protocol where the payload portion is raw JSON.
    for potential_json in extract_json_objects(&text) {
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&potential_json) {
            // Check for base64-encoded bytes envelope.
            if let Some(b64) = event.get("bytes").and_then(|v| v.as_str()) {
                if let Ok(decoded) = base64_decode(b64) {
                    if let Ok(inner) = serde_json::from_slice::<serde_json::Value>(&decoded) {
                        extract_anthropic_event(&inner, &mut content, &mut done);
                    }
                }
                continue;
            }

            // Direct Anthropic event format.
            extract_anthropic_event(&event, &mut content, &mut done);
        }
    }

    if content.is_empty() && !done {
        return None;
    }

    Some(Ok(StreamChunk { content, done }))
}

/// Extract text content and done status from an Anthropic-format streaming
/// event (the same format used by the direct Anthropic API).
fn extract_anthropic_event(event: &serde_json::Value, content: &mut String, done: &mut bool) {
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
            *done = true;
        }
        _ => {}
    }
}

/// Heuristically extract JSON objects from a string that may contain binary
/// data mixed with JSON.  We look for balanced `{` ... `}` sequences.
#[allow(clippy::mut_range_bound)]
fn extract_json_objects(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i;
            let mut depth = 0;
            let mut in_string = false;
            let mut escape_next = false;

            for j in i..chars.len() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                match chars[j] {
                    '\\' if in_string => {
                        escape_next = true;
                    }
                    '"' => {
                        in_string = !in_string;
                    }
                    '{' if !in_string => {
                        depth += 1;
                    }
                    '}' if !in_string => {
                        depth -= 1;
                        if depth == 0 {
                            let obj: String = chars[start..=j].iter().collect();
                            results.push(obj);
                            i = j + 1;
                            break;
                        }
                    }
                    _ => {}
                }
                if j == chars.len() - 1 {
                    i = j + 1;
                }
            }
            if depth != 0 {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    results
}

/// Minimal base64 decoder (standard alphabet, with padding).  Avoids adding a
/// `base64` crate dependency.
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn val(c: u8) -> Result<u8> {
        if let Some(pos) = TABLE.iter().position(|&b| b == c) {
            Ok(pos as u8)
        } else if c == b'=' {
            Ok(0)
        } else {
            Err(anyhow!("invalid base64 character: {}", c as char))
        }
    }

    let input: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !input.len().is_multiple_of(4) {
        return Err(anyhow!("invalid base64 length"));
    }

    let mut output = Vec::with_capacity(input.len() / 4 * 3);

    for chunk in input.chunks(4) {
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = val(chunk[2])?;
        let d = val(chunk[3])?;

        output.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            output.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            output.push((c << 6) | d);
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// AWS credential loading
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct AwsCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

/// Parse an INI-style `~/.aws/credentials` file and extract the given profile.
fn parse_credentials_file(contents: &str, profile: &str) -> Result<AwsCredentials> {
    let target_header = format!("[{}]", profile);
    let mut in_profile = false;
    let mut access_key = None;
    let mut secret_key = None;
    let mut session_token = None;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_profile = trimmed == target_header;
            continue;
        }

        if !in_profile {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "aws_access_key_id" => access_key = Some(value.to_string()),
                "aws_secret_access_key" => secret_key = Some(value.to_string()),
                "aws_session_token" => session_token = Some(value.to_string()),
                _ => {}
            }
        }
    }

    match (access_key, secret_key) {
        (Some(ak), Some(sk)) => Ok(AwsCredentials {
            access_key_id: ak,
            secret_access_key: sk,
            session_token,
        }),
        _ => Err(anyhow!(
            "AWS profile `{}` not found or incomplete in credentials file",
            profile
        )),
    }
}

// ---------------------------------------------------------------------------
// AWS SigV4 signing
// ---------------------------------------------------------------------------

/// Sign an HTTP request using AWS Signature Version 4.
///
/// Returns a list of headers to add to the request (Authorization,
/// X-Amz-Date, and optionally X-Amz-Security-Token).
#[allow(clippy::too_many_arguments)]
fn sign_request_v4(
    method: &str,
    url: &str,
    host: &str,
    body: &[u8],
    region: &str,
    service: &str,
    credentials: &AwsCredentials,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<(String, String)>> {
    let date_stamp = now.format("%Y%m%d").to_string();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

    // --- Step 1: Create canonical request ---

    let payload_hash = hex_encode(&sha256(body));

    // Parse the URL to get the path and query string.
    let (canonical_uri, canonical_querystring) = parse_url_path_and_query(url)?;

    // Signed headers always include host and x-amz-date.
    let mut signed_header_names = vec!["content-type", "host", "x-amz-date"];
    let mut canonical_headers = format!(
        "content-type:application/json\nhost:{}\nx-amz-date:{}\n",
        host, amz_date,
    );

    if credentials.session_token.is_some() {
        signed_header_names.push("x-amz-security-token");
        canonical_headers = format!(
            "content-type:application/json\nhost:{}\nx-amz-date:{}\nx-amz-security-token:{}\n",
            host,
            amz_date,
            credentials.session_token.as_deref().unwrap(),
        );
    }

    let signed_headers = signed_header_names.join(";");

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method,
        canonical_uri,
        canonical_querystring,
        canonical_headers,
        signed_headers,
        payload_hash,
    );

    // --- Step 2: Create string to sign ---

    let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, region, service);

    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        hex_encode(&sha256(canonical_request.as_bytes())),
    );

    // --- Step 3: Calculate signature ---

    let signing_key =
        derive_signing_key(&credentials.secret_access_key, &date_stamp, region, service);

    let signature = hex_encode(&hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    // --- Step 4: Build Authorization header ---

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        credentials.access_key_id, credential_scope, signed_headers, signature,
    );

    let mut headers = vec![
        ("X-Amz-Date".to_string(), amz_date),
        ("Authorization".to_string(), authorization),
        ("X-Amz-Content-Sha256".to_string(), payload_hash),
    ];

    if let Some(ref token) = credentials.session_token {
        headers.push(("X-Amz-Security-Token".to_string(), token.clone()));
    }

    Ok(headers)
}

/// Derive the SigV4 signing key.
///
/// ```text
/// kDate    = HMAC("AWS4" + secret, dateStamp)
/// kRegion  = HMAC(kDate, region)
/// kService = HMAC(kRegion, service)
/// kSigning = HMAC(kService, "aws4_request")
/// ```
fn derive_signing_key(secret: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_secret = format!("AWS4{}", secret);
    let k_date = hmac_sha256(k_secret.as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// Compute HMAC-SHA256 using `ring`.
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let s_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let tag = hmac::sign(&s_key, data);
    tag.as_ref().to_vec()
}

/// Compute SHA-256 hash using `ring`.
fn sha256(data: &[u8]) -> Vec<u8> {
    let d = digest::digest(&digest::SHA256, data);
    d.as_ref().to_vec()
}

/// Hex-encode a byte slice (lowercase).
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Minimal URL-encoding for the model ID path segment.
fn urlencoded(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for c in input.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            _ => {
                for b in c.to_string().as_bytes() {
                    out.push('%');
                    out.push_str(&format!("{:02X}", b));
                }
            }
        }
    }
    out
}

/// Extract the URI path and query string from a full URL.
fn parse_url_path_and_query(url: &str) -> Result<(String, String)> {
    // Strip scheme and host.
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| anyhow!("URL must start with https:// or http://"))?;

    let path_start = without_scheme.find('/').unwrap_or(without_scheme.len());

    let path_and_query = &without_scheme[path_start..];

    if let Some(q) = path_and_query.find('?') {
        Ok((
            path_and_query[..q].to_string(),
            path_and_query[q + 1..].to_string(),
        ))
    } else {
        Ok((path_and_query.to_string(), String::new()))
    }
}

// ---------------------------------------------------------------------------
// Bedrock API types (Anthropic Claude Messages format)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct BedrockRequestBody {
    anthropic_version: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<BedrockMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct BedrockMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct BedrockResponse {
    content: Vec<BedrockContentBlock>,
    #[serde(default)]
    model: Option<String>,
    usage: BedrockUsage,
}

#[derive(Debug, Deserialize)]
struct BedrockContentBlock {
    r#type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct BedrockUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_credentials_file() {
        let contents = "\
[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

[production]
aws_access_key_id = AKIAI44QH8DHBEXAMPLE
aws_secret_access_key = je7MtGbClwBF/2Zp9Utk/h3yCo8nvbEXAMPLEKEY
aws_session_token = FwoGZXIvYXdzEBYaDHqa0AP
";

        let creds = parse_credentials_file(contents, "default").unwrap();
        assert_eq!(creds.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            creds.secret_access_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );
        assert!(creds.session_token.is_none());

        let creds = parse_credentials_file(contents, "production").unwrap();
        assert_eq!(creds.access_key_id, "AKIAI44QH8DHBEXAMPLE");
        assert_eq!(
            creds.session_token.as_deref(),
            Some("FwoGZXIvYXdzEBYaDHqa0AP")
        );

        assert!(parse_credentials_file(contents, "staging").is_err());
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoded("hello-world"), "hello-world");
        assert_eq!(
            urlencoded("anthropic.claude-3-sonnet-20240229-v1:0"),
            "anthropic.claude-3-sonnet-20240229-v1%3A0"
        );
    }

    #[test]
    fn test_parse_url_path_and_query() {
        let (path, qs) = parse_url_path_and_query(
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/invoke",
        )
        .unwrap();
        assert_eq!(path, "/model/foo/invoke");
        assert_eq!(qs, "");

        let (path, qs) =
            parse_url_path_and_query("https://example.com/path?key=value&a=b").unwrap();
        assert_eq!(path, "/path");
        assert_eq!(qs, "key=value&a=b");
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn test_sha256_known_vector() {
        // SHA-256 of empty string.
        let hash = hex_encode(&sha256(b""));
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sigv4_signing_key_derivation() {
        // AWS documentation test vector (not the full signing test, just that
        // the derivation does not panic and produces 32 bytes).
        let key = derive_signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_extract_json_objects() {
        let input =
            r#"some binary junk {"type":"content_block_delta","delta":{"text":"hi"}} more junk"#;
        let objects = extract_json_objects(input);
        assert_eq!(objects.len(), 1);
        assert!(objects[0].contains("content_block_delta"));
    }

    #[test]
    fn test_base64_decode() {
        let decoded = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(decoded, b"Hello");

        let decoded = base64_decode("").unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_endpoint_url() {
        let config = AiProviderConfig {
            region: Some("us-west-2".to_string()),
            model_id: Some("anthropic.claude-3-sonnet-20240229-v1:0".to_string()),
            ..Default::default()
        };
        let provider = BedrockProvider::new(&config);
        let url = provider.endpoint_url();
        assert_eq!(
            url,
            "https://bedrock-runtime.us-west-2.amazonaws.com/model/anthropic.claude-3-sonnet-20240229-v1%3A0/invoke"
        );
    }
}
