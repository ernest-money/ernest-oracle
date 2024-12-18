use crate::events::{EventParams, EventType};
use crate::{IS_SIGNED, PRECISION};
use anyhow::anyhow;
use bitcoin::XOnlyPublicKey;
use kormir::{
    storage::{OracleEventData, Storage},
    EventDescriptor, OracleAnnouncement, OracleAttestation,
};

use serde::{Deserialize, Serialize};

use std::sync::Arc;
use uuid::Uuid;

use crate::OracleState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEvent {
    event_type: EventType,
    maturity: u32,
}

pub async fn create_event_internal(
    state: Arc<OracleState>,
    event: CreateEvent,
) -> anyhow::Result<OracleAnnouncement> {
    let event_id = Uuid::new_v4().to_string();
    let event_params: EventParams = event.event_type.into();
    Ok(state
        .oracle
        .oracle
        .create_numeric_event(
            event_id,
            event_params.nb_digits,
            IS_SIGNED,
            PRECISION,
            event_params.unit,
            event.maturity,
        )
        .await?)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetEvent {
    event_id: String,
}

pub async fn event_internal(
    state: Arc<OracleState>,
    event: GetEvent,
) -> anyhow::Result<Option<OracleAnnouncement>> {
    state
        .oracle
        .oracle
        .storage
        .get_event(event.event_id)
        .await
        .map(|event| event.map(|e| e.announcement))
        .map_err(|e| {
            anyhow!(format!(
                "Failed to retrieve oracle event. error={}",
                e.to_string()
            ))
        })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignEvent {
    event_id: String,
}

pub async fn sign_event_internal(
    state: Arc<OracleState>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct OracleInfo {
    pubkey: XOnlyPublicKey,
    name: String,
}

pub async fn oracle_info_internal(state: Arc<OracleState>) -> OracleInfo {
    OracleInfo {
        pubkey: state.oracle.oracle.public_key(),
        name: "Ernest Hashrate Oracle".to_string(),
    }
}

pub async fn list_events_internal(state: Arc<OracleState>) -> anyhow::Result<Vec<OracleEventData>> {
    Ok(state.oracle.oracle.storage.list_events().await?)
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

    async fn create_oracle() -> Arc<OracleState> {
        let pg_url = std::env::var("DATABASE_URL").unwrap();
        let pool = PgPool::connect(&pg_url).await.unwrap();
        let secp = Secp256k1::new();
        let kormir_key = std::env::var("ERNEST_KEY").unwrap();
        let secret_key = SecretKey::from_str(&kormir_key).unwrap();
        let key_pair = Keypair::from_secret_key(&secp, &secret_key);
        let pubkey = key_pair.x_only_public_key();

        let storage = PostgresStorage::new(pool, pubkey.0).await.unwrap();
        let oracle = ErnestOracle::new(storage, key_pair).unwrap();
        let mempool = MempoolClient::new(BASE_URL.to_string());

        Arc::new(OracleState { oracle, mempool })
    }

    #[tokio::test]
    async fn create_and_sign_hashrate() {
        let oracle = create_oracle().await;

        let request = CreateEvent {
            event_type: EventType::Hashrate,
            maturity: Utc::now().timestamp().try_into().unwrap(),
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
        println!("{:?}", signed_event);
        assert!(signed_event.is_ok())
    }
}
