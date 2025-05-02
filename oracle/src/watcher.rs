use std::{sync::Arc, time::Duration};

use chrono::Utc;
use kormir::{storage::OracleEventData, EventDescriptor};

use crate::{events::EventType, OracleServerState};

pub async fn sign_matured_events_loop(state: Arc<OracleServerState>) {
    let mut timer = tokio::time::interval(Duration::from_secs(60));
    loop {
        timer.tick().await;
        let state_clone = state.clone();
        sign_matured_events(state_clone).await;
    }
}

async fn sign_matured_events(state: Arc<OracleServerState>) {
    let Ok(events) = state.oracle.oracle.storage.list_events().await else {
        return log::error!("Failed to get all events.");
    };

    let now: u32 = Utc::now().timestamp().try_into().unwrap();
    let unsigned_expired_events = events
        .iter()
        .filter(|event| {
            event.announcement.oracle_event.event_maturity_epoch < now
                && event.signatures.is_empty()
        })
        .cloned()
        .collect::<Vec<OracleEventData>>();

    for event in unsigned_expired_events {
        let unit = match event.announcement.oracle_event.event_descriptor {
            EventDescriptor::DigitDecompositionEvent(descriptor) => descriptor.unit,
            EventDescriptor::EnumEvent(_) => continue,
        };

        let Ok(outcome) = EventType::outcome_from_str(&unit, &state.mempool).await else {
            return log::error!("Could not sign for event. event_id={}", event.event_id,);
        };

        if let Err(e) = state
            .oracle
            .oracle
            .sign_numeric_event(event.event_id.clone(), outcome)
            .await
        {
            return log::error!(
                "Could not sign for event. error={} event_id={} outcome={}",
                e.to_string(),
                event.event_id,
                outcome
            );
        }

        return log::info!(
            "Signed event. event_id={} outcome={}",
            event.event_id,
            outcome
        );
    }
}
