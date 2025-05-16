use std::str::FromStr;

use crate::mempool::{MempoolClient, TimePeriod};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, EnumIter, Display, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum EventType {
    Hashrate,
    FeeRate,
    BlockReward,
    DifficultyAdjustment,
}

impl EventType {
    pub async fn outcome_from_str(
        unit: &str,
        mempool_client: &MempoolClient,
    ) -> anyhow::Result<i64> {
        let event_type = EventType::from_str(unit)?;
        let mempool = match event_type {
            EventType::BlockReward => {
                mempool_client
                    .get_block_rewards(TimePeriod::ThreeMonths)
                    .await
            }
            EventType::DifficultyAdjustment => {
                mempool_client
                    .get_difficulty_adjustments(TimePeriod::ThreeMonths)
                    .await
            }
            EventType::FeeRate => mempool_client.get_block_fees(TimePeriod::ThreeMonths).await,
            EventType::Hashrate => mempool_client.get_hashrate(TimePeriod::ThreeMonths).await,
        }?;

        Ok(mempool.ceil() as i64)
    }

    pub async fn outcome(&self, mempool_client: &MempoolClient) -> anyhow::Result<i64> {
        let mempool = match self {
            EventType::BlockReward => {
                mempool_client
                    .get_block_rewards(TimePeriod::ThreeMonths)
                    .await
            }
            EventType::DifficultyAdjustment => {
                mempool_client
                    .get_difficulty_adjustments(TimePeriod::ThreeMonths)
                    .await
            }
            EventType::FeeRate => mempool_client.get_block_fees(TimePeriod::ThreeMonths).await,
            EventType::Hashrate => mempool_client.get_hashrate(TimePeriod::All).await,
        }?;

        Ok(mempool.ceil() as i64)
    }

    pub fn available_events() -> Vec<EventType> {
        EventType::iter().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParams {
    pub event_type: EventType,
    pub nb_digits: u16,
    pub unit: String,
}

impl From<EventType> for EventParams {
    fn from(value: EventType) -> Self {
        match value {
            EventType::BlockReward => Self {
                event_type: value,
                nb_digits: 20,
                unit: EventType::BlockReward.to_string(),
            },
            EventType::DifficultyAdjustment => Self {
                event_type: value,
                nb_digits: 20,
                unit: EventType::DifficultyAdjustment.to_string(),
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
        assert_eq!(&events[1].to_string(), "feerate");
        assert_eq!(&events[2].to_string(), "blockreward");
        assert_eq!(&events[3].to_string(), "difficultyadjustment");
    }
}
