//! Tool trait, registry, and core tools.

pub mod core;
pub mod registry;
pub mod tool;
pub mod types;

pub use registry::ToolRegistry;
pub use tool::{Tool, ToolError};
pub use types::{ToolCall, ToolResult};

pub use crate::core::{
    register_core_tools, BashTool, FileEditTool, FileReadTool, FileWriteTool, GlobTool, GrepTool,
    WebFetchTool, WebSearchTool,
};
