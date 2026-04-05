#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("LLM error: {0}")]
    Llm(#[from] zero_llm::LlmError),

    #[error("Tool error: {0}")]
    Tool(#[from] zero_tools::ToolError),

    #[error("Session aborted")]
    Aborted,

    #[error("Max retries exceeded ({0})")]
    MaxRetries(u32),

    #[error("Context too long")]
    ContextTooLong,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
