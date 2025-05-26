use kormir::EventDescriptor;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;

use crate::{attestation, events::EventType, OracleServerState};

pub async fn sign_matured_events_loop(
    state: Arc<OracleServerState>,
    mut stop_signal: watch::Receiver<bool>,
) {
    let mut timer = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            _ = stop_signal.changed() => {
                if *stop_signal.borrow() {
                    break;
                }
            }
            _ = timer.tick() => {
                sign_matured_events(state.clone()).await;
            }
        }
    }
}

async fn sign_parlay_events(state: Arc<OracleServerState>) {
    let unsiged_matured_parlay_events = state
        .oracle
        .get_matured_unsigned_event_ids_by_type("parlay")
        .await
        .unwrap();

    for (event_id, _) in unsiged_matured_parlay_events {
        if let Err(error) = state.oracle.attest_parlay_contract(event_id.clone()).await {
            log::error!(
                "Failed to attest parlay contract. event_id={} error={}",
                event_id,
                error
            );
            continue;
        }
    }
}

async fn sign_single_events(state: Arc<OracleServerState>) {
    let unsiged_matured_single_events = state
        .oracle
        .get_matured_unsigned_event_ids_by_type("single")
        .await
        .unwrap();

    for (event_id, oracle_event) in unsiged_matured_single_events {
        let unit = match &oracle_event.event_descriptor {
            EventDescriptor::DigitDecompositionEvent(descriptor) => descriptor.unit.clone(),
            EventDescriptor::EnumEvent(_) => continue,
        };
        let Ok(outcome) = EventType::outcome_from_str(&unit, &state.mempool).await else {
            return log::error!("Could not sign for event. event_id={}", event_id);
        };
        if let Err(e) = state
            .oracle
            .oracle
            .sign_numeric_event(event_id.clone(), outcome)
            .await
        {
            return log::error!(
                "Could not sign for event. error={} event_id={} outcome={}",
                e.to_string(),
                event_id,
                outcome
            );
        }

        if let Err(e) = attestation::save_attestation_outcome(
            &state.oracle.oracle.storage.pool,
            event_id.clone(),
            outcome as f64,
            outcome as u64,
        )
        .await
        {
            return log::error!(
                "Could not save attestation outcome. error={} event_id={} outcome={}",
                e.to_string(),
                event_id,
                outcome
            );
        }
        if let Err(e) = attestation::save_attestation_data_outcome(
            &state.oracle.oracle.storage.pool,
            event_id.clone(),
            unit,
            outcome as f64,
            outcome as f64,
        )
        .await
        {
            return log::error!(
                "Could not save attestation data outcome. error={} event_id={} outcome={}",
                e.to_string(),
                event_id,
                outcome
            );
        }

        return log::info!("Signed event. event_id={} outcome={}", event_id, outcome);
    }
}

async fn sign_matured_events(state: Arc<OracleServerState>) {
    sign_parlay_events(state.clone()).await;
    sign_single_events(state.clone()).await;
}
