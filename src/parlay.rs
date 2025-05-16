use crate::events::EventType;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::prelude::FromRow;
use sqlx::PgPool;
use sqlx::Row;
use std::str::FromStr;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParlayParameter {
    /// The type of event to be monitored from Bitcoin core
    pub data_type: EventType,
    /// The threshold value for the event for contract strike
    pub threshold: u64,
    /// The range of the data type
    pub range: u64,
    /// Whether the event is above the threshold for contract strike
    pub is_above_threshold: bool,
    /// The transformation function to be applied to the event
    pub transformation: TransformationFunction,
    /// The weight of the event
    pub weight: f64,
}

impl ParlayParameter {
    pub fn normalize_parameter(&self, value: i64) -> f64 {
        if self.is_above_threshold {
            // Parameter must EXCEED threshold (e.g., hash rate > X)
            if value <= self.threshold as i64 {
                // Below threshold - return 0
                return 0.0;
            } else {
                // Above threshold - normalize based on distance
                let distance = value - self.threshold as i64;
                let normalized = (distance as f64) / (self.range as f64);
                // Cap at 1.0 for values beyond threshold + range
                return normalized.min(1.0);
            }
        } else {
            // Parameter must STAY BELOW threshold (e.g., price < Y)
            if value >= self.threshold as i64 {
                // Above threshold - return 0
                return 0.0;
            } else {
                // Below threshold - normalize based on distance
                let distance = self.threshold as i64 - value;
                let normalized = (distance as f64) / (self.range as f64);
                // Cap at 1.0 for values beyond threshold - range
                return normalized.min(1.0);
            }
        }
    }

    pub fn apply_transformation(&self, normalized_value: f64) -> f64 {
        match self.transformation {
            TransformationFunction::Linear => normalized_value,
            TransformationFunction::Quadratic => normalized_value * normalized_value,
            TransformationFunction::Sqrt => normalized_value.sqrt(),
            TransformationFunction::Exponential => normalized_value.exp(),
            TransformationFunction::Logarithmic => normalized_value.ln(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, EnumIter, Display, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TransformationFunction {
    Linear,
    Quadratic,
    Sqrt,
    Exponential,
    Logarithmic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumIter, Display, EnumString)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum CombinationMethod {
    Multiply,
    WeightedAverage,
    GeometricMean,
    Min,
    Max,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct ParlayContract {
    /// The id of the contract used for the announcement
    pub id: String,
    /// The set of parameters of the contract
    pub parameters: Vec<ParlayParameter>,
    /// The method used to combine the events
    pub combination_method: CombinationMethod,
    /// The maximum normalized value for the contract
    pub max_normalized_value: u64, // Scale for attestation (e.g., 1000 [.34 -> 340])
}

impl ParlayContract {
    pub async fn new(
        pool: PgPool,
        id: String,
        parameters: Vec<ParlayParameter>,
        combination_method: CombinationMethod,
        max_normalized_value: u64,
    ) -> anyhow::Result<Self> {
        // Start a transaction
        let mut tx = pool.begin().await?;

        // Insert the main contract
        sqlx::query(
            "INSERT INTO parlay_contracts (id, combination_method, max_normalized_value) 
         VALUES ($1, $2, $3)",
        )
        .bind(&id)
        .bind(combination_method.to_string())
        .bind(max_normalized_value as i64)
        .execute(&mut *tx)
        .await?;

        // Insert each parameter
        for param in &parameters {
            sqlx::query(
                "INSERT INTO parlay_parameters 
             (contract_id, data_type, threshold, range, is_above_threshold, transformation, weight) 
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&id)
            .bind(param.data_type.to_string())
            .bind(param.threshold as i64)
            .bind(param.range as i64)
            .bind(param.is_above_threshold)
            .bind(param.transformation.to_string())
            .bind(param.weight as f64)
            .execute(&mut *tx)
            .await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(Self {
            id,
            parameters,
            combination_method,
            max_normalized_value,
        })
    }
}

pub async fn get_parlay_contract(pool: PgPool, id: String) -> anyhow::Result<ParlayContract> {
    let contract = sqlx::query("SELECT * FROM parlay_contracts WHERE id = $1")
        .bind(&id)
        .fetch_one(&pool)
        .await?;

    let parameters = sqlx::query("SELECT * FROM parlay_parameters WHERE contract_id = $1")
        .bind(&id)
        .fetch_all(&pool)
        .await?;

    contract_from_row(contract, parameters)
}

fn contract_from_row(contract: PgRow, parameters: Vec<PgRow>) -> anyhow::Result<ParlayContract> {
    let id: String = contract.try_get("id").expect("id not found");
    let combination_method = {
        let row: String = contract.get("combination_method");
        CombinationMethod::from_str(&row)?
    };
    let max_normalized_value = {
        let row: i64 = contract.get("max_normalized_value");
        row as u64
    };

    let parameters = parameters
        .iter()
        .map(|p| parlay_parameter_from_row(p))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ParlayContract {
        id,
        parameters,
        combination_method,
        max_normalized_value,
    })
}

fn parlay_parameter_from_row(row: &PgRow) -> anyhow::Result<ParlayParameter> {
    let data_type: String = row.get("data_type");
    let threshold: i64 = row.get("threshold");
    let range: i64 = row.get("range");
    let is_above_threshold: bool = row.get("is_above_threshold");
    let transformation: String = row.get("transformation");
    let weight: f64 = row.get("weight");

    Ok(ParlayParameter {
        data_type: EventType::from_str(&data_type)?,
        threshold: threshold as u64,
        range: range as u64,
        is_above_threshold,
        transformation: TransformationFunction::from_str(&transformation)?,
        weight,
    })
}

pub fn combine_scores(
    events: &[f64],
    weights: &[f64],
    combination_method: &CombinationMethod,
) -> f64 {
    match combination_method {
        CombinationMethod::Multiply => events.iter().product(),
        CombinationMethod::WeightedAverage => {
            let sum: f64 = events.iter().zip(weights).map(|(e, w)| e * w).sum();
            let total_weight: f64 = weights.iter().sum();
            sum / total_weight
        }
        CombinationMethod::GeometricMean => {
            let product: f64 = events.iter().product();
            product.powf(1.0 / events.len() as f64)
        }
        CombinationMethod::Min => {
            if events.is_empty() {
                0.0
            } else {
                events.iter().copied().fold(f64::INFINITY, f64::min)
            }
        }
        CombinationMethod::Max => events.iter().copied().fold(0.0, f64::max),
    }
}

pub fn convert_to_attestable_value(combined_score: f64, max_normalized_value: u64) -> u64 {
    (combined_score * max_normalized_value as f64) as u64
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::*;

    #[tokio::test]
    async fn test_parlay_contract() {
        let pool =
            PgPool::connect(&std::env::var("DATABASE_URL").expect("$DATABASE_URL is not set"))
                .await
                .unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let _ = ParlayContract::new(
            pool,
            id,
            vec![
                ParlayParameter {
                    data_type: EventType::Hashrate,
                    threshold: 1000,
                    range: 1000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.0,
                },
                ParlayParameter {
                    data_type: EventType::Hashrate,
                    threshold: 1000,
                    range: 1000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.3,
                },
            ],
            CombinationMethod::Multiply,
            1000,
        )
        .await
        .expect("could not create parlay contract");
    }

    #[test]
    fn transformation_conversion() {
        let trans = TransformationFunction::iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>();
        assert_eq!(trans.len(), 5);
        assert_eq!(trans[0], "linear");
        assert_eq!(trans[1], "quadratic");
        assert_eq!(trans[2], "sqrt");
        assert_eq!(trans[3], "exponential");
        assert_eq!(trans[4], "logarithmic");
    }

    #[test]
    fn combination_method_conversion() {
        let comb = CombinationMethod::iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>();
        assert_eq!(comb.len(), 5);
        assert_eq!(comb[0], "multiply");
        assert_eq!(comb[1], "weightedAverage");
        assert_eq!(comb[2], "geometricMean");
        assert_eq!(comb[3], "min");
        assert_eq!(comb[4], "max");
    }
}
