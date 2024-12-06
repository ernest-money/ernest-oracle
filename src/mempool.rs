use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

const BASE_URL: &str = "https://mempool.space/api/v1";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashratePeriod {
    pub timestamp: i64,
    pub avg_hashrate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DifficultyPeriod {
    pub time: i64,
    pub difficulty: f64,
    pub height: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashrateResponse {
    pub hashrates: Vec<HashratePeriod>,
    pub difficulty: Vec<DifficultyPeriod>,
    pub current_hashrate: f64,
    pub current_difficulty: f64,
}

pub struct MempoolClient {
    client: Client,
}

#[derive(Debug)]
pub enum TimePeriod {
    OneMonth,
    ThreeMonths,
    SixMonths,
    OneYear,
    TwoYears,
    ThreeYears,
    All,
}

impl TimePeriod {
    fn as_str(&self) -> &'static str {
        match self {
            TimePeriod::OneMonth => "1m",
            TimePeriod::ThreeMonths => "3m",
            TimePeriod::SixMonths => "6m",
            TimePeriod::OneYear => "1y",
            TimePeriod::TwoYears => "2y",
            TimePeriod::ThreeYears => "3y",
            TimePeriod::All => "",
        }
    }
}

impl MempoolClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_hashrate(&self, period: TimePeriod) -> Result<i64, Box<dyn Error>> {
        let url = match period {
            TimePeriod::All => format!("{}/mining/hashrate", BASE_URL),
            _ => format!("{}/mining/hashrate/{}", BASE_URL, period.as_str()),
        };

        let response = self.client.get(&url).send().await?;
        let data = response.json::<HashrateResponse>().await?;
        let hashrate = data.current_hashrate.ceil() as i64;
        let terra_hashes_per_second = hashrate / 10_i64.pow(12);
        Ok(terra_hashes_per_second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_get_hashrate() {
        let client = MempoolClient::new();
        let result = client.get_hashrate(TimePeriod::ThreeMonths).await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data > 0);
    }
}
