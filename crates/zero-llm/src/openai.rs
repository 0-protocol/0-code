//! OpenAI-compatible streaming provider.
//!
//! Implements the `/v1/chat/completions` SSE streaming protocol used by OpenAI,
//! Flock, and many other compatible endpoints.

use std::env;

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::error::LlmError;
use crate::provider::ModelProvider;
use crate::types::{ModelParams, StreamEvent, Usage};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_ENV_VAR: &str = "ZERO_CODE_API_KEY";
const DEFAULT_MAX_CONTEXT: u64 = 128_000;

/// Configuration for an OpenAI-compatible endpoint.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub env_var: String,
    pub auth_header: String,
    pub auth_prefix: String,
    pub max_context_tokens: u64,
    pub provider_name: String,
    pub default_model: String,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            env_var: DEFAULT_ENV_VAR.to_string(),
            auth_header: "authorization".to_string(),
            auth_prefix: "Bearer ".to_string(),
            max_context_tokens: DEFAULT_MAX_CONTEXT,
            provider_name: "openai".to_string(),
            default_model: "gpt-4o".to_string(),
        }
    }
}

impl OpenAiConfig {
    /// Preset for Flock AI API platform (<https://docs.flock.io>).
    pub fn flock() -> Self {
        Self {
            base_url: "https://api.flock.io/v1".to_string(),
            env_var: "FLOCK_API_KEY".to_string(),
            auth_header: "x-litellm-api-key".to_string(),
            auth_prefix: String::new(),
            max_context_tokens: 128_000,
            provider_name: "flock".to_string(),
            default_model: "qwen3-30b-a3b-instruct-2507".to_string(),
        }
    }
}

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
    config: OpenAiConfig,
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_config(api_key, OpenAiConfig::default())
    }

    pub fn with_config(api_key: String, config: OpenAiConfig) -> Self {
        let model = config.default_model.clone();
        Self {
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("failed to build HTTP client"),
            api_key,
            model,
            config,
        }
    }

    /// Create a Flock-configured provider from the `FLOCK_API_KEY` env var.
    pub fn flock_from_env() -> Result<Self, LlmError> {
        let cfg = OpenAiConfig::flock();
        let api_key = env::var(&cfg.env_var)
            .map_err(|_| LlmError::Auth(format!("{} not set", cfg.env_var)))?;
        Ok(Self::with_config(api_key, cfg))
    }

    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = env::var(DEFAULT_ENV_VAR)
            .map_err(|_| LlmError::Auth(format!("{DEFAULT_ENV_VAR} not set")))?;
        Ok(Self::new(api_key))
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = url.into();
        self
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
    }

    fn build_request_body(&self, params: &ModelParams) -> serde_json::Value {
        let model = if params.model.is_empty() || params.model == "default" {
            &self.model
        } else {
            &params.model
        };

        let mut messages: Vec<serde_json::Value> = Vec::new();

        if let Some(ref system) = params.system {
            messages.push(json!({ "role": "system", "content": system }));
        }

        for msg in &params.messages {
            let role_str = match msg.role {
                crate::types::Role::System => "system",
                crate::types::Role::User => "user",
                crate::types::Role::Assistant => "assistant",
            };

            let content_blocks = &msg.content;
            if content_blocks.len() == 1 {
                match &content_blocks[0] {
                    crate::types::ContentBlock::Text { text } => {
                        messages.push(json!({ "role": role_str, "content": text }));
                    }
                    crate::types::ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => {
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_use_id,
                            "content": content,
                        }));
                    }
                    crate::types::ContentBlock::ToolUse { id, name, input } => {
                        messages.push(json!({
                            "role": "assistant",
                            "tool_calls": [{
                                "id": id,
                                "type": "function",
                                "function": { "name": name, "arguments": input.to_string() }
                            }]
                        }));
                    }
                    crate::types::ContentBlock::Thinking { .. } => {}
                }
            } else {
                let mut text_parts = Vec::new();
                let mut tool_calls_out = Vec::new();
                let mut tool_results = Vec::new();

                for block in content_blocks {
                    match block {
                        crate::types::ContentBlock::Text { text } => {
                            text_parts.push(text.clone());
                        }
                        crate::types::ContentBlock::ToolUse { id, name, input } => {
                            tool_calls_out.push(json!({
                                "id": id,
                                "type": "function",
                                "function": { "name": name, "arguments": input.to_string() }
                            }));
                        }
                        crate::types::ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            ..
                        } => {
                            tool_results.push((tool_use_id.clone(), content.clone()));
                        }
                        crate::types::ContentBlock::Thinking { .. } => {}
                    }
                }

                if !text_parts.is_empty() || !tool_calls_out.is_empty() {
                    let mut msg_obj = json!({ "role": role_str });
                    if !text_parts.is_empty() {
                        msg_obj["content"] = json!(text_parts.join(""));
                    }
                    if !tool_calls_out.is_empty() {
                        msg_obj["tool_calls"] = json!(tool_calls_out);
                    }
                    messages.push(msg_obj);
                }

                for (tid, tcontent) in tool_results {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tid,
                        "content": tcontent,
                    }));
                }
            }
        }

        let mut body = json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "max_tokens": params.max_tokens,
        });

        if let Some(temp) = params.temperature {
            body["temperature"] = json!(temp);
        }

        if !params.stop_sequences.is_empty() {
            body["stop"] = json!(params.stop_sequences);
        }

        if !params.tools.is_empty() {
            let tools: Vec<serde_json::Value> = params
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect();
            body["tools"] = json!(tools);
        }

        body
    }

    async fn process_stream(
        &self,
        response: reqwest::Response,
        tx: &mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, LlmError> {
        let mut usage = Usage::default();
        let mut tool_accumulators: Vec<ToolAccumulator> = Vec::new();
        let mut buf = String::new();
        let mut byte_buf: Vec<u8> = Vec::new();

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| LlmError::Stream(e.to_string()))?;
            byte_buf.extend_from_slice(&chunk);

            let valid_up_to = match std::str::from_utf8(&byte_buf) {
                Ok(_) => byte_buf.len(),
                Err(e) => e.valid_up_to(),
            };
            if valid_up_to > 0 {
                let text =
                    std::str::from_utf8(&byte_buf[..valid_up_to]).expect("validated UTF-8 slice");
                buf.push_str(text);
                byte_buf.drain(..valid_up_to);
            }

            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim().to_string();
                buf = buf[pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                let data = if let Some(d) = line.strip_prefix("data: ") {
                    d.trim()
                } else {
                    continue;
                };

                if data == "[DONE]" {
                    let _ = tx.send(StreamEvent::EndTurn).await;
                    for tool in tool_accumulators.drain(..) {
                        let input: serde_json::Value = if tool.arguments.is_empty() {
                            json!({})
                        } else {
                            serde_json::from_str(&tool.arguments).unwrap_or_else(|e| {
                                warn!("failed to parse tool call arguments: {e}");
                                json!({})
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
                    return Ok(usage);
                }

                let json: serde_json::Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(e) => {
                        debug!(error = %e, "skipping unparseable SSE data");
                        continue;
                    }
                };

                if let Some(err) = json.get("error") {
                    let msg = err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown error")
                        .to_string();
                    let _ = tx.send(StreamEvent::Error(msg.clone())).await;
                    return Err(LlmError::Api {
                        status: 0,
                        message: msg,
                    });
                }

                if let Some(u) = json.get("usage") {
                    if let Some(pt) = u.get("prompt_tokens").and_then(|v| v.as_u64()) {
                        usage.input_tokens = pt;
                        let _ = tx.send(StreamEvent::InputTokens(pt)).await;
                    }
                    if let Some(ct) = u.get("completion_tokens").and_then(|v| v.as_u64()) {
                        usage.output_tokens = ct;
                        let _ = tx.send(StreamEvent::OutputTokens(ct)).await;
                    }
                }

                let Some(choices) = json.get("choices").and_then(|c| c.as_array()) else {
                    continue;
                };

                for choice in choices {
                    let Some(delta) = choice.get("delta") else {
                        continue;
                    };

                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        if !content.is_empty() {
                            let _ = tx.send(StreamEvent::TextDelta(content.to_string())).await;
                        }
                    }

                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                        for tc in tool_calls {
                            let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;

                            while tool_accumulators.len() <= idx {
                                tool_accumulators.push(ToolAccumulator::default());
                            }

                            if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                tool_accumulators[idx].id = id.to_string();
                            }
                            if let Some(func) = tc.get("function") {
                                if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                    tool_accumulators[idx].name = name.to_string();
                                    let _ = tx
                                        .send(StreamEvent::ToolUse {
                                            id: tool_accumulators[idx].id.clone(),
                                            name: name.to_string(),
                                            input: json!({}),
                                        })
                                        .await;
                                }
                                if let Some(args) =
                                    func.get("arguments").and_then(|a| a.as_str())
                                {
                                    tool_accumulators[idx].arguments.push_str(args);
                                }
                            }
                        }
                    }

                    if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                        if reason == "tool_calls" {
                            for tool in tool_accumulators.drain(..) {
                                let input: serde_json::Value = if tool.arguments.is_empty() {
                                    json!({})
                                } else {
                                    serde_json::from_str(&tool.arguments).unwrap_or_else(|e| {
                                        warn!("failed to parse tool call arguments: {e}");
                                        json!({})
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
                    }
                }
            }
        }

        Ok(usage)
    }
}

#[derive(Debug, Default, Clone)]
struct ToolAccumulator {
    id: String,
    name: String,
    arguments: String,
}

fn error_for_status(status: u16, body: &str) -> LlmError {
    if status == 401 || status == 403 {
        return LlmError::Auth(body.to_string());
    }
    if status == 429 {
        let ms = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get("error")?.get("message")?.as_str().map(String::from))
            .and_then(|s| s.split_whitespace().find_map(|w| w.parse::<u64>().ok()))
            .unwrap_or(60_000);
        return LlmError::RateLimited { retry_after_ms: ms };
    }
    LlmError::Api {
        status,
        message: body.to_string(),
    }
}

#[async_trait]
impl ModelProvider for OpenAiProvider {
    fn name(&self) -> &str {
        &self.config.provider_name
    }

    async fn stream(
        &self,
        params: ModelParams,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, LlmError> {
        let body = self.build_request_body(&params);
        let url = self.chat_completions_url();

        let auth_value = format!("{}{}", self.config.auth_prefix, self.api_key);

        let response = self
            .client
            .post(&url)
            .header(&self.config.auth_header, &auth_value)
            .header("content-type", "application/json")
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
    fn test_flock_config() {
        let cfg = OpenAiConfig::flock();
        assert_eq!(cfg.base_url, "https://api.flock.io/v1");
        assert_eq!(cfg.auth_header, "x-litellm-api-key");
        assert!(cfg.auth_prefix.is_empty());
        assert_eq!(cfg.provider_name, "flock");
        assert_eq!(cfg.default_model, "qwen3-30b-a3b-instruct-2507");
    }

    #[test]
    fn test_chat_completions_url() {
        let p = OpenAiProvider::new("key".into());
        assert_eq!(
            p.chat_completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );

        let p = OpenAiProvider::with_config("key".into(), OpenAiConfig::flock());
        assert_eq!(
            p.chat_completions_url(),
            "https://api.flock.io/v1/chat/completions"
        );
    }

    #[test]
    fn test_build_simple_request() {
        let p = OpenAiProvider::with_config("key".into(), OpenAiConfig::flock());
        let params = ModelParams {
            model: "qwen3-30b-a3b-instruct-2507".into(),
            messages: vec![crate::types::Message {
                role: crate::types::Role::User,
                content: vec![crate::types::ContentBlock::Text {
                    text: "Hello".into(),
                }],
            }],
            system: Some("You are helpful.".into()),
            tools: vec![],
            max_tokens: 1024,
            temperature: Some(0.7),
            stop_sequences: vec![],
        };
        let body = p.build_request_body(&params);
        assert_eq!(body["model"], "qwen3-30b-a3b-instruct-2507");
        assert_eq!(body["stream"], true);
        assert_eq!(body["max_tokens"], 1024);
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "Hello");
    }

    #[test]
    fn test_build_request_with_tools() {
        let p = OpenAiProvider::new("key".into());
        let params = ModelParams {
            model: String::new(),
            messages: vec![crate::types::Message {
                role: crate::types::Role::User,
                content: vec![crate::types::ContentBlock::Text {
                    text: "What's the weather?".into(),
                }],
            }],
            system: None,
            tools: vec![crate::types::ToolDefinition {
                name: "get_weather".into(),
                description: "Get weather for a city".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    }
                }),
            }],
            max_tokens: 4096,
            temperature: None,
            stop_sequences: vec![],
        };
        let body = p.build_request_body(&params);
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "get_weather");
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

    #[test]
    fn test_default_model_override() {
        let p = OpenAiProvider::new("key".into()).with_model("gpt-4o-mini");
        assert_eq!(p.model, "gpt-4o-mini");
    }

    #[test]
    fn test_flock_provider_creation() {
        let p = OpenAiProvider::with_config("sk-test".into(), OpenAiConfig::flock())
            .with_model("qwen3-30b-a3b-instruct-2507");
        assert_eq!(p.config.provider_name, "flock");
        assert_eq!(p.model, "qwen3-30b-a3b-instruct-2507");
    }
}
