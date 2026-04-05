//! SSE-streaming Messages API provider.
//!
//! Implements the SSE streaming protocol used by many LLM providers.
//! Configurable API URL and authentication headers.

use std::env;

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::error::LlmError;
use crate::provider::ModelProvider;
use crate::types::{ModelParams, StreamEvent, Usage};

const DEFAULT_API_URL: &str = "https://api.openrouter.ai/api/v1/messages";
const DEFAULT_API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "default";
const DEFAULT_MAX_CONTEXT_TOKENS: u64 = 200_000;
const DEFAULT_ENV_VAR: &str = "ZERO_CODE_API_KEY";

/// Provider configuration for SSE-streaming Messages API endpoints.
#[derive(Debug, Clone)]
pub struct MessagesApiConfig {
    pub api_url: String,
    pub api_version: String,
    pub auth_header: String,
    pub version_header: Option<String>,
    pub env_var: String,
    pub max_context_tokens: u64,
}

impl Default for MessagesApiConfig {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            api_version: DEFAULT_API_VERSION.to_string(),
            auth_header: "authorization".to_string(),
            version_header: None,
            env_var: DEFAULT_ENV_VAR.to_string(),
            max_context_tokens: DEFAULT_MAX_CONTEXT_TOKENS,
        }
    }
}

pub struct MessagesApiProvider {
    client: Client,
    api_key: String,
    default_model: String,
    config: MessagesApiConfig,
}

impl MessagesApiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("failed to build HTTP client"),
            api_key,
            default_model: DEFAULT_MODEL.to_string(),
            config: MessagesApiConfig::default(),
        }
    }

    pub fn with_config(mut self, config: MessagesApiConfig) -> Self {
        self.config = config;
        self
    }

    pub fn from_env() -> Result<Self, LlmError> {
        Self::from_env_var(DEFAULT_ENV_VAR)
    }

    pub fn from_env_var(var: &str) -> Result<Self, LlmError> {
        let api_key = env::var(var)
            .map_err(|_| LlmError::Auth(format!("{var} not set")))?;
        Ok(Self::new(api_key))
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub fn with_api_url(mut self, url: impl Into<String>) -> Self {
        self.config.api_url = url.into();
        self
    }

    fn build_request_body(&self, params: &ModelParams) -> serde_json::Value {
        let model = if params.model.is_empty() {
            &self.default_model
        } else {
            &params.model
        };

        let messages: Vec<serde_json::Value> = params
            .messages
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": msg.role,
                    "content": msg.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": params.max_tokens,
            "messages": messages,
            "stream": true,
        });

        if let Some(ref system) = params.system {
            body["system"] = serde_json::json!(system);
        }

        if !params.tools.is_empty() {
            let tools: Vec<serde_json::Value> = params
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tools);
        }

        if let Some(temp) = params.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if !params.stop_sequences.is_empty() {
            body["stop_sequences"] = serde_json::json!(params.stop_sequences);
        }

        body
    }

    async fn process_stream(
        &self,
        response: reqwest::Response,
        tx: &mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, LlmError> {
        let mut usage = Usage::default();
        let mut current_tool: Option<ToolAccumulator> = None;
        let mut buf = String::new();

        let mut stream = response.bytes_stream();
        let mut byte_buf: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| LlmError::Stream(e.to_string()))?;
            byte_buf.extend_from_slice(&chunk);
            let valid_up_to = match std::str::from_utf8(&byte_buf) {
                Ok(_) => byte_buf.len(),
                Err(e) => e.valid_up_to(),
            };
            if valid_up_to > 0 {
                let text = std::str::from_utf8(&byte_buf[..valid_up_to])
                    .expect("validated UTF-8 slice");
                buf.push_str(text);
                byte_buf.drain(..valid_up_to);
            }

            while let Some(pos) = buf.find("\n\n") {
                let event_block = buf[..pos].to_string();
                buf = buf[pos + 2..].to_string();

                if let Some((event_type, data)) = parse_sse_event(&event_block) {
                    self.handle_event(&event_type, &data, tx, &mut usage, &mut current_tool)
                        .await?;
                }
            }
        }

        if !buf.trim().is_empty() {
            if let Some((event_type, data)) = parse_sse_event(&buf) {
                self.handle_event(&event_type, &data, tx, &mut usage, &mut current_tool)
                    .await?;
            }
        }

        Ok(usage)
    }

    async fn handle_event(
        &self,
        event_type: &str,
        data: &str,
        tx: &mpsc::Sender<StreamEvent>,
        usage: &mut Usage,
        current_tool: &mut Option<ToolAccumulator>,
    ) -> Result<(), LlmError> {
        let json: serde_json::Value =
            serde_json::from_str(data).map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        match event_type {
            "message_start" => {
                if let Some(u) = json.get("message").and_then(|m| m.get("usage")) {
                    if let Some(input) = u.get("input_tokens").and_then(|v| v.as_u64()) {
                        usage.input_tokens = input;
                        let _ = tx.send(StreamEvent::InputTokens(input)).await;
                    }
                    if let Some(cache_create) = u
                        .get("cache_creation_input_tokens")
                        .and_then(|v| v.as_u64())
                    {
                        usage.cache_creation_input_tokens = cache_create;
                    }
                    if let Some(cache_read) =
                        u.get("cache_read_input_tokens").and_then(|v| v.as_u64())
                    {
                        usage.cache_read_input_tokens = cache_read;
                    }
                }
            }

            "content_block_start" => {
                if let Some(cb) = json.get("content_block") {
                    match cb.get("type").and_then(|t| t.as_str()) {
                        Some("tool_use") => {
                            let id = cb
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let name = cb
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            *current_tool = Some(ToolAccumulator {
                                id,
                                name,
                                input_json: String::new(),
                            });
                        }
                        Some("thinking") => {
                            debug!("thinking block started");
                        }
                        Some("text") => {
                            debug!("text block started");
                        }
                        other => {
                            debug!(?other, "unknown content block type");
                        }
                    }
                }
            }

            "content_block_delta" => {
                if let Some(delta) = json.get("delta") {
                    match delta.get("type").and_then(|t| t.as_str()) {
                        Some("text_delta") => {
                            if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                let _ = tx.send(StreamEvent::TextDelta(text.to_string())).await;
                            }
                        }
                        Some("thinking_delta") => {
                            if let Some(thinking) =
                                delta.get("thinking").and_then(|v| v.as_str())
                            {
                                let _ = tx
                                    .send(StreamEvent::ThinkingDelta(thinking.to_string()))
                                    .await;
                            }
                        }
                        Some("input_json_delta") => {
                            if let Some(partial) =
                                delta.get("partial_json").and_then(|v| v.as_str())
                            {
                                if let Some(ref mut tool) = current_tool {
                                    tool.input_json.push_str(partial);
                                }
                            }
                        }
                        other => {
                            debug!(?other, "unknown delta type");
                        }
                    }
                }
            }

            "content_block_stop" => {
                if let Some(tool) = current_tool.take() {
                    let input: serde_json::Value = if tool.input_json.is_empty() {
                        serde_json::json!({})
                    } else {
                        serde_json::from_str(&tool.input_json).unwrap_or_else(|e| {
                            warn!("failed to parse tool input JSON: {e}");
                            serde_json::json!({})
                        })
                    };
                    let _ = tx
                        .send(StreamEvent::ToolUse {
                            id: tool.id,
                            name: tool.name,
                            input,
                        })
                        .await;
                }
            }

            "message_delta" => {
                if let Some(u) = json.get("usage") {
                    if let Some(output) = u.get("output_tokens").and_then(|v| v.as_u64()) {
                        usage.output_tokens = output;
                        let _ = tx.send(StreamEvent::OutputTokens(output)).await;
                    }
                }
            }

            "message_stop" => {
                let _ = tx.send(StreamEvent::EndTurn).await;
            }

            "ping" => {}

            "error" => {
                let msg = json
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                let _ = tx.send(StreamEvent::Error(msg.clone())).await;
                return Err(LlmError::Api {
                    status: 0,
                    message: msg,
                });
            }

            _ => {
                debug!(event_type, "unhandled SSE event type");
            }
        }

        Ok(())
    }
}

struct ToolAccumulator {
    id: String,
    name: String,
    input_json: String,
}

fn parse_sse_event(block: &str) -> Option<(String, String)> {
    let mut event_type = String::new();
    let mut data_lines = Vec::new();

    for line in block.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            event_type = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data: ") {
            data_lines.push(value);
        }
    }

    if event_type.is_empty() || data_lines.is_empty() {
        return None;
    }

    Some((event_type, data_lines.join("\n")))
}

fn error_for_status(status: u16, body: &str) -> LlmError {
    if status == 401 || status == 403 {
        return LlmError::Auth(body.to_string());
    }

    if status == 429 {
        let retry_after = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get("error")?.get("message")?.as_str().map(String::from))
            .unwrap_or_default();

        let ms = retry_after
            .split_whitespace()
            .find_map(|w| w.parse::<u64>().ok())
            .unwrap_or(60_000);

        return LlmError::RateLimited { retry_after_ms: ms };
    }

    LlmError::Api {
        status,
        message: body.to_string(),
    }
}

#[async_trait]
impl ModelProvider for MessagesApiProvider {
    fn name(&self) -> &str {
        "messages-api"
    }

    async fn stream(
        &self,
        params: ModelParams,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, LlmError> {
        let body = self.build_request_body(&params);

        let mut request = self
            .client
            .post(&self.config.api_url)
            .header(&self.config.auth_header, format!("Bearer {}", &self.api_key))
            .header("content-type", "application/json");

        if let Some(ref vh) = self.config.version_header {
            request = request.header(vh.as_str(), &self.config.api_version);
        }

        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "failed to read error body".into());
            return Err(error_for_status(status, &body_text));
        }

        self.process_stream(response, &tx).await
    }

    fn max_context_tokens(&self) -> u64 {
        self.config.max_context_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_event() {
        let block = "event: message_start\ndata: {\"type\":\"message_start\"}";
        let (event, data) = parse_sse_event(block).unwrap();
        assert_eq!(event, "message_start");
        assert_eq!(data, "{\"type\":\"message_start\"}");
    }

    #[test]
    fn test_parse_sse_event_empty() {
        assert!(parse_sse_event("").is_none());
        assert!(parse_sse_event("event: ping").is_none());
        assert!(parse_sse_event("data: {}").is_none());
    }

    #[test]
    fn test_error_for_status_auth() {
        match error_for_status(401, "unauthorized") {
            LlmError::Auth(msg) => assert_eq!(msg, "unauthorized"),
            other => panic!("expected Auth, got {other:?}"),
        }
    }

    #[test]
    fn test_error_for_status_rate_limit() {
        match error_for_status(429, r#"{"error":{"message":"retry after 5000 ms"}}"#) {
            LlmError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 5000),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }
}
