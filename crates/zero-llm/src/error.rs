#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Context too long: {tokens} tokens exceeds {max} limit")]
    ContextTooLong { tokens: u64, max: u64 },

    #[error("Provider not supported: {0}")]
    Unsupported(String),

    #[error("Stream error: {0}")]
    Stream(String),
}
