//! 0-Protocol integration: on-chain operations, 0-dex/0-ads integration, and
//! cryptographic verification. Types mirror 0-lang concepts for future bridging.

mod ads;
mod dex;
mod error;
mod oracle;
mod types;
mod verify;

pub use ads::{AdCampaign, AdsClient, CampaignStatus};
pub use dex::{DexClient, DexOrder, DexOrderResult, OrderSide, OrderType};
pub use error::ProtocolError;
pub use oracle::OracleClient;
pub use types::{ChainOp, ChainResult, Tensor, TensorData};
pub use verify::OracleVerifier;
