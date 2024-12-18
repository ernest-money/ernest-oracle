use crate::mempool::{MempoolClient, TimePeriod};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Hashrate,
    FeeRate,
    BlockReward,
    DificultyAdjustment,
}

impl TryFrom<&str> for EventType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "sats/block" => Ok(Self::BlockReward),
            "difficulty" => Ok(Self::DificultyAdjustment),
            "fee-rate" => Ok(Self::FeeRate),
            "H/s" => Ok(Self::Hashrate),
            _ => Err(anyhow!("Unit not supported.".to_string())),
        }
    }
}

impl EventType {
    pub async fn outcome_from_str(
        unit: &str,
        mempool_client: &MempoolClient,
    ) -> anyhow::Result<i64> {
        let event_type: EventType = unit.try_into()?;
        let mempool = match event_type {
            EventType::BlockReward => {
                mempool_client
                    .get_block_rewards(TimePeriod::ThreeMonths)
                    .await
            }
            EventType::DificultyAdjustment => {
                mempool_client
                    .get_difficulty_adjustments(TimePeriod::ThreeMonths)
                    .await
            }
            EventType::FeeRate => mempool_client.get_block_fees(TimePeriod::ThreeMonths).await,
            EventType::Hashrate => mempool_client.get_hashrate(TimePeriod::ThreeMonths).await,
        }?;

        Ok(mempool.ceil() as i64)
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
                unit: "sats/block".to_string(),
            },
            EventType::DificultyAdjustment => Self {
                event_type: value,
                nb_digits: 20,
                unit: "difficulty".to_string(),
            },
            EventType::FeeRate => Self {
                event_type: value,
                nb_digits: 20,
                unit: "fee-rate".to_string(),
            },
            EventType::Hashrate => Self {
                event_type: value,
                nb_digits: 20,
                unit: "H/s".to_string(),
            },
        }
    }
}
