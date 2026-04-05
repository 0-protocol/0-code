use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use zero_core::{AgentEvent, EngineConfig, QueryEngine};
use zero_llm::{ContentBlock, LlmError, ModelParams, ModelProvider, StreamEvent, Usage};
use zero_tools::ToolRegistry;

struct MockProvider {
    response: String,
}

#[async_trait]
impl ModelProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn stream(
        &self,
        _params: ModelParams,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, LlmError> {
        let _ = tx.send(StreamEvent::TextDelta(self.response.clone())).await;
        let _ = tx.send(StreamEvent::EndTurn).await;
        Ok(Usage::default())
    }

    fn max_context_tokens(&self) -> u64 {
        200_000
    }
}

#[tokio::test]
async fn test_simple_text_response() {
    let provider = Arc::new(MockProvider {
        response: "Hello!".into(),
    });
    let registry = ToolRegistry::new();
    let tools = Arc::new(registry);
    let config = EngineConfig::default();

    let mut engine = QueryEngine::new(provider, tools, config);
    let (tx, mut rx) = mpsc::channel(256);

    engine.run("Hi", tx).await.unwrap();

    let mut text = String::new();
    while let Ok(event) = rx.try_recv() {
        if let AgentEvent::TextDelta(t) = event {
            text.push_str(&t);
        }
    }
    assert_eq!(text, "Hello!");
}

struct MockToolProvider;

#[async_trait]
impl ModelProvider for MockToolProvider {
    fn name(&self) -> &str {
        "mock-tools"
    }

    async fn stream(
        &self,
        params: ModelParams,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, LlmError> {
        let has_tool_results = params
            .messages
            .iter()
            .any(|m| m.content.iter().any(|c| matches!(c, ContentBlock::ToolResult { .. })));

        if has_tool_results {
            let _ = tx.send(StreamEvent::TextDelta("Done!".into())).await;
        } else {
            let _ = tx
                .send(StreamEvent::ToolUse {
                    id: "t1".into(),
                    name: "file_read".into(),
                    input: serde_json::json!({"path": "/tmp/test.txt"}),
                })
                .await;
        }
        let _ = tx.send(StreamEvent::EndTurn).await;
        Ok(Usage::default())
    }

    fn max_context_tokens(&self) -> u64 {
        200_000
    }
}

#[tokio::test]
async fn test_tool_use_loop() {
    let provider = Arc::new(MockToolProvider);
    let mut registry = ToolRegistry::new();
    zero_tools::register_core_tools(&mut registry);
    let tools = Arc::new(registry);
    let config = EngineConfig::default();

    let mut engine = QueryEngine::new(provider, tools, config);
    let (tx, mut rx) = mpsc::channel(256);

    engine.run("Read a file", tx).await.unwrap();

    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }

    assert!(events
        .iter()
        .any(|e| matches!(e, AgentEvent::ToolStart { name, .. } if name == "file_read")));
    assert!(events
        .iter()
        .any(|e| matches!(e, AgentEvent::TextDelta(t) if t == "Done!")));
}
