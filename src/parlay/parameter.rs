use crate::events::EventType;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::prelude::FromRow;
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

pub fn parlay_parameter_from_row(row: &PgRow) -> anyhow::Result<ParlayParameter> {
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

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::parlay::contract::CombinationMethod;

    use super::*;

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
