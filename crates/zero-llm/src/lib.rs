//! LLM provider abstraction.
//!
//! Provides a unified [`ModelProvider`] trait for streaming LLM completions:
//!
//! - [`OpenAiProvider`] — full OpenAI-compatible streaming (also powers Flock, OpenRouter, etc.)
//! - [`MessagesApiProvider`] — SSE Messages API streaming
//!
//! Use [`OpenAiConfig::flock()`] for a ready-made Flock preset.

pub mod error;
pub mod messages_api;
pub mod openai;
pub mod provider;
pub mod types;

pub use error::LlmError;
pub use messages_api::{MessagesApiConfig, MessagesApiProvider};
pub use openai::{OpenAiConfig, OpenAiProvider};
pub use provider::ModelProvider;
pub use types::{ContentBlock, Message, ModelParams, Role, StreamEvent, ToolDefinition, Usage};
