//! MCP (Model Context Protocol) client for connecting to tool servers.

mod client;
mod stdio;
mod transport;
mod types;

pub use client::{McpClient, McpError};
pub use transport::McpTransport;
pub use types::{McpResource, McpServerConfig, McpTool, TransportType};
