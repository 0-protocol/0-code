use serde::{Deserialize, Serialize};

use crate::ProtocolError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdCampaign {
    pub name: String,
    pub budget: String,
    pub target_audience: Vec<String>,
    pub content: String,
    pub status: CampaignStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CampaignStatus {
    Draft,
    Active,
    Paused,
    Completed,
}

pub struct AdsClient {
    #[allow(dead_code)]
    endpoint: Option<String>,
}

impl AdsClient {
    pub fn new(endpoint: Option<String>) -> Self {
        Self { endpoint }
    }

    pub async fn create_campaign(&self, _campaign: &AdCampaign) -> Result<String, ProtocolError> {
        Err(ProtocolError::NotImplemented(
            "AdsClient::create_campaign — connect to 0-ads".into(),
        ))
    }

    pub async fn get_campaign(&self, id: &str) -> Result<AdCampaign, ProtocolError> {
        let _ = id;
        Err(ProtocolError::NotImplemented("campaign fetch".into()))
    }
}
