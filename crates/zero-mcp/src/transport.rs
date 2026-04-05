use crate::McpError;
use async_trait::async_trait;

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send(&self, message: serde_json::Value) -> Result<serde_json::Value, McpError>;
    async fn close(&self) -> Result<(), McpError>;
}
