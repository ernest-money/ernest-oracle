use crate::events::{EventParams, EventType};
use crate::parlay::{CombinationMethod, ParlayContract, ParlayParameter};
use crate::OracleServerError;
use crate::OracleServerState;
use anyhow::anyhow;
use bitcoin::XOnlyPublicKey;
use kormir::{
    storage::{OracleEventData, Storage},
    EventDescriptor, OracleAnnouncement, OracleAttestation,
};

use serde::{Deserialize, Serialize};

use std::sync::Arc;
use uuid::Uuid;

pub const IS_SIGNED: bool = false;
pub const PRECISION: i32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CreateEvent {
    Single {
        event_type: EventType,
        maturity: u32,
    },
    Parlay {
        parameters: Vec<ParlayParameter>,
        combination_method: CombinationMethod,
        max_normalized_value: Option<u64>,
        event_maturity_epoch: u32,
    },
}

pub async fn create_event_internal(
    state: Arc<OracleServerState>,
    event: CreateEvent,
) -> anyhow::Result<OracleAnnouncement> {
    let announcement = match event {
        CreateEvent::Single {
            event_type,
            maturity,
        } => {
            let event_id = Uuid::new_v4().to_string();
            let event_params: EventParams = event_type.into();
            Ok(state
                .oracle
                .oracle
                .create_numeric_event(
                    event_id,
                    event_params.nb_digits,
                    IS_SIGNED,
                    PRECISION,
                    event_params.unit,
                    maturity,
                )
                .await?)
        }
        CreateEvent::Parlay {
            parameters,
            combination_method,
            max_normalized_value,
            event_maturity_epoch,
        } => {
            let announcement = state
                .oracle
                .create_parlay_announcement(
                    parameters,
                    combination_method,
                    max_normalized_value,
                    event_maturity_epoch,
                )
                .await?;
            Ok(announcement)
        }
    };
    announcement
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetAnnouncement {
    event_id: String,
}

pub async fn get_announcement_internal(
    state: Arc<OracleServerState>,
    event: GetAnnouncement,
) -> Result<OracleAnnouncement, OracleServerError> {
    Ok(state
        .oracle
        .oracle
        .storage
        .get_event(event.event_id)
        .await
        .map_err(|e| OracleServerError {
            reason: e.to_string(),
        })?
        .ok_or(OracleServerError {
            reason: "Announcement not found".to_string(),
        })?
        .announcement)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignEvent {
    pub event_id: String,
}

pub async fn sign_event_internal(
    state: Arc<OracleServerState>,
    event: SignEvent,
) -> anyhow::Result<OracleAttestation> {
    let event = state
        .oracle
        .oracle
        .storage
        .get_event(event.event_id)
        .await?;

    let Some(event) = event else {
        return Err(anyhow!("Event does not exist.".to_string()));
    };

    let unit = match event.announcement.oracle_event.event_descriptor {
        EventDescriptor::DigitDecompositionEvent(descriptor) => descriptor.unit,
        EventDescriptor::EnumEvent(_) => {
            return Err(anyhow!("Cannot sign enum descriptor.".to_string()))
        }
    };

    let outcome = EventType::outcome_from_str(&unit, &state.mempool).await?;

    Ok(state
        .oracle
        .oracle
        .sign_numeric_event(event.event_id, outcome)
        .await?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAttestation {
    event_id: String,
}

pub async fn get_attestation_internal(
    state: Arc<OracleServerState>,
    event: GetAttestation,
) -> anyhow::Result<OracleAttestation> {
    let event = match state
        .oracle
        .oracle
        .storage
        .get_event(event.event_id)
        .await?
    {
        Some(e) => e,
        None => return Err(anyhow!("Could not find event.")),
    };

    if event.signatures.is_empty() {
        return Err(anyhow!("Event is not signed."));
    } else {
        Ok(OracleAttestation {
            event_id: event.event_id,
            oracle_public_key: event.announcement.oracle_public_key,
            signatures: event.signatures.iter().cloned().map(|sig| sig.1).collect(),
            outcomes: event.signatures.iter().cloned().map(|o| o.0).collect(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OracleInfo {
    pub pubkey: XOnlyPublicKey,
    pub name: String,
}

pub async fn oracle_info_internal(state: Arc<OracleServerState>) -> OracleInfo {
    OracleInfo {
        pubkey: state.oracle.oracle.public_key(),
        name: "Ernest Hashrate Oracle".to_string(),
    }
}

pub async fn list_events_internal(
    state: Arc<OracleServerState>,
) -> anyhow::Result<Vec<OracleEventData>> {
    let events = state.oracle.oracle.storage.list_events().await?;
    Ok(events)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetParlayContract {
    pub event_id: String,
}

pub async fn get_parlay_contract_internal(
    state: Arc<OracleServerState>,
    event: GetParlayContract,
) -> anyhow::Result<ParlayContract> {
    Ok(state.oracle.get_parlay_contract(event.event_id).await?)
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bitcoin::{
        key::{Keypair, Secp256k1},
        secp256k1::SecretKey,
    };
    use chrono::Utc;
    use sqlx::PgPool;

    use crate::{
        mempool::{MempoolClient, BASE_URL},
        oracle::ErnestOracle,
        storage::PostgresStorage,
    };

    use super::*;

    async fn create_oracle() -> Arc<OracleServerState> {
        let pg_url = std::env::var("DATABASE_URL").unwrap();
        let pool = PgPool::connect(&pg_url).await.unwrap();
        let secp = Secp256k1::new();
        let kormir_key = std::env::var("ERNEST_KEY").unwrap();
        let secret_key = SecretKey::from_str(&kormir_key).unwrap();
        let key_pair = Keypair::from_secret_key(&secp, &secret_key);
        let pubkey = key_pair.x_only_public_key();

        let storage = PostgresStorage::new(pool.clone(), pubkey.0, false)
            .await
            .unwrap();
        let mempool = MempoolClient::new(BASE_URL.to_string());
        let oracle = ErnestOracle::new(storage, pool, key_pair, mempool.clone()).unwrap();

        Arc::new(OracleServerState { oracle, mempool })
    }

    #[tokio::test]
    async fn create_and_sign_hashrate() {
        let oracle = create_oracle().await;
        let timestamp = Utc::now().timestamp() + 10000;

        let request = CreateEvent::Single {
            event_type: EventType::Hashrate,
            maturity: timestamp as u32,
        };
        let event = create_event_internal(oracle.clone(), request)
            .await
            .unwrap();

        let event_id = event.oracle_event.event_id.clone();

        let signed_event = oracle
            .oracle
            .oracle
            .sign_numeric_event(event_id, 400_000)
            .await;
        assert!(signed_event.is_ok())
    }
}
