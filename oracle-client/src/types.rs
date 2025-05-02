use bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OracleError {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Hashrate,
    FeeRate,
    BlockReward,
    DificultyAdjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEvent {
    pub event_type: EventType,
    pub maturity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetEvent {
    pub event_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignEvent {
    pub event_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OracleInfo {
    pub pubkey: XOnlyPublicKey,
    pub name: String,
}
