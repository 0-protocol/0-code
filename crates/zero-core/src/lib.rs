//! Core agent loop engine.
//!
//! Provides [`QueryEngine`], the async agentic loop that ties an LLM
//! [`ModelProvider`](zero_llm::ModelProvider) to a
//! [`ToolRegistry`](zero_tools::ToolRegistry), streaming
//! [`AgentEvent`]s to the caller as it works.

pub mod config;
pub mod engine;
pub mod error;
pub mod session;

pub use config::EngineConfig;
pub use engine::{AgentEvent, QueryEngine};
pub use error::EngineError;
pub use session::Session;
