use std::{sync::Arc, time::Duration};

use chrono::Utc;
use kormir::storage::OracleEventData;

use crate::{mempool::TimePeriod, OracleState};

pub async fn sign_matured_events(state: Arc<OracleState>) {
    let mut timer = tokio::time::interval(Duration::from_secs(60));
    loop {
        timer.tick().await;
        let Ok(events) = state.oracle.oracle.storage.list_events().await else {
            log::error!("Failed to get all events.");
            continue;
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

        let current_hashrate = state
            .mempool
            .get_hashrate(TimePeriod::ThreeMonths)
            .await
            .unwrap();
        for event in unsigned_expired_events {
            if let Err(e) = state
                .oracle
                .oracle
                .sign_numeric_event(event.event_id.clone(), current_hashrate)
                .await
            {
                log::info!(
                    "Could not sign for event. error={} event_id={} hashrate={}",
                    e.to_string(),
                    event.event_id,
                    current_hashrate
                );
                continue;
            }

            log::info!(
                "Signed event. event_id={} hashrate={}",
                event.event_id,
                current_hashrate
            );
        }
    }
}

fn sleep() {
    std::thread::sleep(Duration::from_secs(60))
}
