use crate::attestation::ErnestOracleOutcome;
use crate::events::EventType;
use crate::parlay::{
    contract::{CombinationMethod, ParlayContract},
    parameter::ParlayParameter,
};
use crate::OracleServerState;
use crate::{attestation, OracleServerError};
use anyhow::anyhow;
use bitcoin::XOnlyPublicKey;
use kormir::{
    storage::{OracleEventData, Storage},
    EventDescriptor, OracleAnnouncement, OracleAttestation,
};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CreateEvent {
    Single {
        #[serde(rename = "eventType")]
        event_type: EventType,
        maturity: u32,
    },
    Parlay {
        parameters: Vec<ParlayParameter>,
        #[serde(rename = "combinationMethod")]
        combination_method: CombinationMethod,
        #[serde(rename = "maxNormalizedValue")]
        max_normalized_value: Option<u64>,
        #[serde(rename = "eventMaturityEpoch")]
        event_maturity_epoch: u32,
    },
}

pub async fn create_event_internal(
    state: Arc<OracleServerState>,
    event: CreateEvent,
) -> anyhow::Result<OracleAnnouncement> {
    state.oracle.create_event(event).await
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
        name: "Ernest Parlay Oracle".to_string(),
    }
}

pub async fn list_events_internal(
    state: Arc<OracleServerState>,
) -> anyhow::Result<Vec<OracleEventData>> {
    let events = state.oracle.oracle.storage.oracle_event_data().await?;
    Ok(events)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetParlayContract {
    pub event_id: String,
}

pub async fn get_parlay_contract_internal(
    state: Arc<OracleServerState>,
    event: GetParlayContract,
) -> anyhow::Result<ParlayContract> {
    Ok(state.oracle.get_parlay_contract(event.event_id).await?)
}

pub fn get_available_events_internal() -> Vec<EventType> {
    EventType::available_events()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAttestationOutcome {
    pub event_id: String,
}

pub async fn get_attestation_outcome_internal(
    state: Arc<OracleServerState>,
    event: GetAttestationOutcome,
) -> anyhow::Result<ErnestOracleOutcome> {
    Ok(
        attestation::get_attestation_outcome(&state.oracle.oracle.storage.pool, event.event_id)
            .await?,
    )
}
