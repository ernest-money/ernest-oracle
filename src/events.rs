use std::str::FromStr;

use crate::mempool::{MempoolClient, TimePeriod};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, EnumIter, Display, EnumString)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum EventType {
    Hashrate,
    FeeRate,
    BlockFees,
    Difficulty,
}

impl EventType {
    pub async fn outcome_from_str(
        unit: &str,
        mempool_client: &MempoolClient,
    ) -> anyhow::Result<i64> {
        let event_type = EventType::from_str(unit)?;
        let mempool = match event_type {
            EventType::BlockFees => mempool_client.get_block_fees(TimePeriod::ThreeMonths).await,
            EventType::Difficulty => mempool_client.get_difficulty(TimePeriod::ThreeMonths).await,
            EventType::FeeRate => mempool_client.get_fee_rate(TimePeriod::ThreeMonths).await,
            EventType::Hashrate => mempool_client.get_hashrate(TimePeriod::ThreeMonths).await,
        }?;

        Ok(mempool.ceil() as i64)
    }

    /// OK, we need floating points!!!!
    pub async fn outcome(&self, mempool_client: &MempoolClient) -> anyhow::Result<i64> {
        let mempool = match self {
            EventType::BlockFees => mempool_client.get_block_fees(TimePeriod::ThreeMonths).await,
            EventType::Difficulty => mempool_client.get_difficulty(TimePeriod::ThreeMonths).await,
            EventType::FeeRate => mempool_client.get_fee_rate(TimePeriod::ThreeMonths).await,
            EventType::Hashrate => mempool_client.get_hashrate(TimePeriod::ThreeMonths).await,
        }?;

        Ok(mempool.ceil() as i64)
    }

    pub fn available_events() -> Vec<EventType> {
        EventType::iter().collect()
    }
}

/// Parameters for an event.
///
/// This is used to store the event type, the number of digits to round to, and the unit of the event.
/// Specifically when the event is a single contract to be attested to.
///
/// The unit is used to determine the unit of the event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParams {
    pub event_type: EventType,
    pub nb_digits: u16,
    pub unit: String,
}

/// TODO: get the updates params for the data set
impl From<EventType> for EventParams {
    fn from(value: EventType) -> Self {
        match value {
            EventType::BlockFees => Self {
                event_type: value,
                nb_digits: 20,
                unit: EventType::BlockFees.to_string(),
            },
            EventType::Difficulty => Self {
                event_type: value,
                nb_digits: 20,
                unit: EventType::Difficulty.to_string(),
            },
            EventType::FeeRate => Self {
                event_type: value,
                nb_digits: 20,
                unit: EventType::FeeRate.to_string(),
            },
            EventType::Hashrate => Self {
                event_type: value,
                nb_digits: 20,
                unit: EventType::Hashrate.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_events() {
        let events = EventType::available_events();
        assert_eq!(events.len(), 4);
        assert_eq!(&events[0].to_string(), "hashrate");
        assert_eq!(&events[1].to_string(), "feeRate");
        assert_eq!(&events[2].to_string(), "blockFees");
        assert_eq!(&events[3].to_string(), "difficulty");
    }
}
