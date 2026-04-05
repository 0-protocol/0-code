use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, warn};
use zero_llm::{ContentBlock, ModelParams, ModelProvider, StreamEvent, ToolDefinition};
use zero_tools::{ToolCall, ToolRegistry};

use crate::{EngineConfig, EngineError, Session};

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolStart { id: String, name: String },
    ToolEnd { id: String, result: String, is_error: bool },
    TurnComplete { usage_input: u64, usage_output: u64 },
    Error(String),
}

pub struct QueryEngine {
    provider: Arc<dyn ModelProvider>,
    tools: Arc<ToolRegistry>,
    config: EngineConfig,
    session: Session,
}

impl QueryEngine {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        tools: Arc<ToolRegistry>,
        config: EngineConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            config,
            session: Session::new(),
        }
    }

    /// Run the agent loop for a single user message.
    ///
    /// Streams [`AgentEvent`]s to `tx` as the loop progresses through
    /// LLM completions and tool executions. Returns once the LLM produces
    /// a final response with no further tool calls, or a terminal condition
    /// (max turns / max retries) is reached.
    pub async fn run(
        &mut self,
        message: &str,
        tx: mpsc::Sender<AgentEvent>,
    ) -> Result<(), EngineError> {
        self.session.add_user_message(message);
        let mut retries = 0u32;

        loop {
            if self.session.turn_count >= self.config.max_turns {
                debug!(turns = self.session.turn_count, "Max turns reached");
                let _ = tx
                    .send(AgentEvent::Error("Max turns reached".into()))
                    .await;
                break;
            }

            let tool_defs: Vec<ToolDefinition> = self
                .tools
                .list_definitions()
                .into_iter()
                .filter_map(|v| match serde_json::from_value::<ToolDefinition>(v.clone()) {
                    Ok(td) => Some(td),
                    Err(e) => {
                        warn!(error = %e, "Skipping tool with invalid definition");
                        None
                    }
                })
                .collect();

            let params = ModelParams {
                model: self.config.model.clone(),
                messages: self.session.messages.clone(),
                system: self.config.system_prompt.clone(),
                tools: tool_defs,
                max_tokens: self.config.max_tokens,
                temperature: self.config.temperature,
                stop_sequences: vec![],
            };

            // Spawn the provider stream so we can consume events as they arrive.
            let (llm_tx, mut llm_rx) = mpsc::channel::<StreamEvent>(256);
            let provider = self.provider.clone();
            let stream_handle = tokio::spawn(async move { provider.stream(params, llm_tx).await });

            let mut text_parts: Vec<String> = Vec::new();
            let mut tool_uses: Vec<ToolCall> = Vec::new();

            while let Some(event) = llm_rx.recv().await {
                match event {
                    StreamEvent::TextDelta(t) => {
                        if tx.send(AgentEvent::TextDelta(t.clone())).await.is_err() {
                            return Err(EngineError::Aborted);
                        }
                        text_parts.push(t);
                    }
                    StreamEvent::ThinkingDelta(t) => {
                        if tx.send(AgentEvent::ThinkingDelta(t)).await.is_err() {
                            return Err(EngineError::Aborted);
                        }
                    }
                    StreamEvent::ToolUse { id, name, input } => {
                        if tx
                            .send(AgentEvent::ToolStart {
                                id: id.clone(),
                                name: name.clone(),
                            })
                            .await
                            .is_err()
                        {
                            return Err(EngineError::Aborted);
                        }
                        tool_uses.push(ToolCall { id, name, input });
                    }
                    StreamEvent::Error(e) => {
                        let _ = tx.send(AgentEvent::Error(e)).await;
                    }
                    StreamEvent::InputTokens(_)
                    | StreamEvent::OutputTokens(_)
                    | StreamEvent::EndTurn => {}
                }
            }

            let usage = match stream_handle.await {
                Ok(Ok(u)) => u,
                Ok(Err(llm_err)) => {
                    retries += 1;
                    warn!(retries, error = %llm_err, "LLM stream failed");
                    if retries > self.config.max_retries {
                        return Err(EngineError::MaxRetries(self.config.max_retries));
                    }
                    let _ = tx.send(AgentEvent::Error(llm_err.to_string())).await;
                    let backoff = Duration::from_millis(500 * 2u64.pow(retries.min(6)));
                    tokio::time::sleep(backoff).await;
                    continue;
                }
                Err(join_err) => {
                    return Err(EngineError::Other(anyhow::anyhow!(
                        "Stream task panicked: {join_err}"
                    )));
                }
            };

            self.session.accumulate_usage(&usage);
            self.session.turn_count += 1;
            retries = 0;

            // Build the assistant message from collected text + tool-use blocks.
            let mut blocks = Vec::new();
            if !text_parts.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: text_parts.join(""),
                });
            }
            for tc in &tool_uses {
                blocks.push(ContentBlock::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.input.clone(),
                });
            }
            self.session.add_assistant_message(blocks);

            if tool_uses.is_empty() {
                let _ = tx
                    .send(AgentEvent::TurnComplete {
                        usage_input: usage.input_tokens,
                        usage_output: usage.output_tokens,
                    })
                    .await;
                break;
            }

            // Execute tool calls and feed results back into the session.
            let results = self.tools.execute_batch(tool_uses).await;
            let mut tool_result_data = Vec::new();
            for (id, result) in results {
                let _ = tx
                    .send(AgentEvent::ToolEnd {
                        id: id.clone(),
                        result: result.content.clone(),
                        is_error: result.is_error,
                    })
                    .await;
                tool_result_data.push((id, result.content, result.is_error));
            }
            self.session.add_tool_results(tool_result_data);
        }

        Ok(())
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}
