use std::{fmt::Display, str::FromStr};

use crate::mempool::{MempoolClient, TimePeriod};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventType {
    Hashrate,
    FeeRate,
    BlockReward,
    DificultyAdjustment,
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Hashrate => write!(f, "hashrate"),
            EventType::FeeRate => write!(f, "fee-rate"),
            EventType::BlockReward => write!(f, "block-reward"),
            EventType::DificultyAdjustment => write!(f, "difficulty"),
        }
    }
}

impl FromStr for EventType {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "block-reward" => Ok(Self::BlockReward),
            "difficulty" => Ok(Self::DificultyAdjustment),
            "fee-rate" => Ok(Self::FeeRate),
            "hashrate" => Ok(Self::Hashrate),
            _ => Err(anyhow!("Unit not supported.".to_string())),
        }
    }
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

    pub async fn outcome(&self, mempool_client: &MempoolClient) -> anyhow::Result<i64> {
        let mempool = match self {
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
            EventType::Hashrate => mempool_client.get_hashrate(TimePeriod::All).await,
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
                unit: "block-reward".to_string(),
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
                unit: "hashrate".to_string(),
            },
        }
    }
}
