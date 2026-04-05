use serde::{Deserialize, Serialize};

/// Represents a tensor value with confidence (mirroring 0-lang)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tensor {
    pub data: TensorData,
    pub confidence: f64,
    pub shape: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TensorData {
    Float(Vec<f64>),
    String(String),
    /// Serialized decimal
    Decimal(String),
    Json(serde_json::Value),
}

impl Tensor {
    pub fn from_string(s: String, confidence: f64) -> Self {
        Self {
            shape: Vec::new(),
            data: TensorData::String(s),
            confidence,
        }
    }

    pub fn from_json(v: serde_json::Value, confidence: f64) -> Self {
        Self {
            shape: Vec::new(),
            data: TensorData::Json(v),
            confidence,
        }
    }

    pub fn from_float(v: f64, confidence: f64) -> Self {
        Self {
            shape: vec![1],
            data: TensorData::Float(vec![v]),
            confidence,
        }
    }
}

/// On-chain operation types (mirroring 0-lang Op variants)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainOp {
    OracleRead {
        chain_id: u64,
        contract: String,
        calldata: String,
    },
    GetGasPrice {
        chain_id: u64,
    },
    SendTransaction {
        chain_id: u64,
        to: String,
        data: String,
        value: String,
    },
}

/// Result of an on-chain operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainResult {
    pub data: Tensor,
    pub tx_hash: Option<String>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u64>,
}
