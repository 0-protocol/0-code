use crate::{ChainResult, ProtocolError, Tensor};

pub struct OracleClient {
    rpc_urls: std::collections::HashMap<u64, String>, // chain_id -> rpc_url
    client: reqwest::Client,
}

impl OracleClient {
    pub fn new() -> Self {
        Self {
            rpc_urls: std::collections::HashMap::new(),
            client: reqwest::Client::new(),
        }
    }

    pub fn add_rpc(&mut self, chain_id: u64, url: String) {
        self.rpc_urls.insert(chain_id, url);
    }

    /// Execute an eth_call read
    pub async fn oracle_read(
        &self,
        chain_id: u64,
        contract: &str,
        calldata: &str,
    ) -> Result<ChainResult, ProtocolError> {
        let rpc_url = self
            .rpc_urls
            .get(&chain_id)
            .ok_or(ProtocolError::ChainNotConfigured(chain_id))?;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": contract,
                "data": calldata,
            }, "latest"],
            "id": 1
        });

        let resp = self
            .client
            .post(rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProtocolError::RpcError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProtocolError::RpcError(e.to_string()))?;

        if let Some(error) = json.get("error") {
            return Err(ProtocolError::RpcError(error.to_string()));
        }

        let result = json["result"]
            .as_str()
            .ok_or_else(|| ProtocolError::InvalidResponse("missing result".into()))?;

        Ok(ChainResult {
            data: Tensor::from_string(result.to_string(), 1.0),
            tx_hash: None,
            block_number: None,
            gas_used: None,
        })
    }

    /// Get current gas price (EIP-1559)
    pub async fn get_gas_price(&self, chain_id: u64) -> Result<ChainResult, ProtocolError> {
        let rpc_url = self
            .rpc_urls
            .get(&chain_id)
            .ok_or(ProtocolError::ChainNotConfigured(chain_id))?;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 1
        });

        let resp = self
            .client
            .post(rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProtocolError::RpcError(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProtocolError::RpcError(e.to_string()))?;

        let gas_price = json["result"]
            .as_str()
            .ok_or_else(|| ProtocolError::InvalidResponse("missing gas price".into()))?;

        Ok(ChainResult {
            data: Tensor::from_string(gas_price.to_string(), 1.0),
            tx_hash: None,
            block_number: None,
            gas_used: None,
        })
    }
}

impl Default for OracleClient {
    fn default() -> Self {
        Self::new()
    }
}
