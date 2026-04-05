use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::LlmError;
use crate::types::{ModelParams, StreamEvent, Usage};

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn stream(
        &self,
        params: ModelParams,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<Usage, LlmError>;

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> u64;
}
