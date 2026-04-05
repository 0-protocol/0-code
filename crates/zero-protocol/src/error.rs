#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Chain {0} not configured")]
    ChainNotConfigured(u64),
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
