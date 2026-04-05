use serde::{Deserialize, Serialize};

use crate::ProtocolError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexOrder {
    pub pair: String, // e.g. "ETH/USDC"
    pub side: OrderSide,
    pub amount: String,
    /// None for market orders
    pub price: Option<String>,
    pub order_type: OrderType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexOrderResult {
    pub order_id: String,
    pub status: String,
    pub filled_amount: Option<String>,
    pub average_price: Option<String>,
}

pub struct DexClient {
    #[allow(dead_code)]
    endpoint: Option<String>,
}

impl DexClient {
    pub fn new(endpoint: Option<String>) -> Self {
        Self { endpoint }
    }

    pub async fn place_order(&self, _order: &DexOrder) -> Result<DexOrderResult, ProtocolError> {
        Err(ProtocolError::NotImplemented(
            "DexClient::place_order — connect to 0-dex".into(),
        ))
    }

    pub async fn get_order_status(&self, _order_id: &str) -> Result<DexOrderResult, ProtocolError> {
        Err(ProtocolError::NotImplemented(
            "DexClient::get_order_status — connect to 0-dex".into(),
        ))
    }
}
