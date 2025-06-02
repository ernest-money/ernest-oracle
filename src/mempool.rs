use reqwest::Client;
use serde::{Deserialize, Serialize};

pub const BASE_URL: &str = "https://mempool.space/api/v1";

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

#[derive(Debug, Serialize)]
pub struct DifficultyAdjustment {
    pub timestamp: i64,
    pub height: i64,
    pub difficulty: f64,
    pub difficulty_change: f64,
}

// Custom deserialization for the array format
impl<'de> Deserialize<'de> for DifficultyAdjustment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let arr = <[f64; 4]>::deserialize(deserializer)?;
        Ok(DifficultyAdjustment {
            timestamp: arr[0] as i64,
            height: arr[1] as i64,
            difficulty: arr[2],
            difficulty_change: arr[3],
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockFees {
    pub avg_height: i64,
    pub timestamp: i64,
    pub avg_fees: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeRate {
    pub avg_height: i64,
    pub timestamp: i64,
    #[serde(rename = "avgFee_0")]
    pub avg_fee_0: f64,
    #[serde(rename = "avgFee_10")]
    pub avg_fee_10: f64,
    #[serde(rename = "avgFee_25")]
    pub avg_fee_25: f64,
    #[serde(rename = "avgFee_50")]
    pub avg_fee_50: f64,
    #[serde(rename = "avgFee_75")]
    pub avg_fee_75: f64,
    #[serde(rename = "avgFee_90")]
    pub avg_fee_90: f64,
    #[serde(rename = "avgFee_100")]
    pub avg_fee_100: f64,
}

#[derive(Debug, Clone)]
pub struct MempoolClient {
    client: Client,
    base_url: String,
}

/// TODO: do we need to get the latest fee or the average over a time period?
impl MempoolClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn get_hashrate(&self, period: TimePeriod) -> anyhow::Result<f64> {
        let url = match period {
            TimePeriod::All => format!("{}/mining/hashrate", self.base_url),
            _ => format!("{}/mining/hashrate/{}", self.base_url, period.as_str()),
        };

        let response = self.client.get(&url).send().await?;
        let data = response.json::<HashrateResponse>().await?;
        Ok(data.current_hashrate / 1e18)
    }

    pub async fn get_block_fees(&self, period: TimePeriod) -> anyhow::Result<f64> {
        let url = format!("{}/mining/blocks/fees/{}", self.base_url, period.as_str());
        let response = self.client.get(&url).send().await?;
        let data = response.json::<Vec<BlockFees>>().await?;
        let average_fees = Self::calculate_average(data, |f| f.avg_fees as f64);
        Ok(average_fees)
    }

    pub async fn get_difficulty(&self, interval: TimePeriod) -> anyhow::Result<f64> {
        let url = format!("{}/mining/hashrate/{}", self.base_url, interval.as_str());

        let response = self.client.get(&url).send().await?;
        let data = response.json::<HashrateResponse>().await?;
        Ok(data.current_difficulty / 1e12)
    }

    pub async fn get_fee_rate(&self, period: TimePeriod) -> anyhow::Result<f64> {
        let url = format!(
            "{}/mining/blocks/fee-rates/{}",
            self.base_url,
            period.as_str()
        );
        let response = self.client.get(&url).send().await?;
        let data = response.json::<Vec<FeeRate>>().await?;
        let average_fee_rate = Self::calculate_average(data, |f| f.avg_fee_90);
        Ok(average_fee_rate)
    }

    fn calculate_average<T, F>(data: Vec<T>, extractor: F) -> f64
    where
        F: Fn(&T) -> f64,
    {
        let total: f64 = data.iter().map(&extractor).sum();
        total / data.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::MempoolClient;
    use super::*;
    use crate::test_util::setup_mock_server;

    #[tokio::test]
    async fn test_mempool_client() {
        let client = MempoolClient::new(BASE_URL.to_string());

        // Test hashrate endpoint
        let hashrate = client.get_hashrate(TimePeriod::ThreeMonths).await.unwrap();
        assert!(hashrate > 0.0);

        // Test block fees endpoint
        let fees = client.get_block_fees(TimePeriod::ThreeMonths).await;
        assert!(fees.unwrap() > 0.0);

        // Test difficulty adjustments endpoint
        let difficulty = client
            .get_difficulty(TimePeriod::ThreeMonths)
            .await
            .unwrap();
        assert!(difficulty > 0.0);

        // Test fee rate endpoint
        let fee_rate = client.get_fee_rate(TimePeriod::ThreeMonths).await.unwrap();
        assert!(fee_rate > 0.0);
    }

    #[tokio::test]
    async fn test_mempool_client_with_mock_server() {
        // Start mock server
        let mock_server = setup_mock_server().await;

        // Create client with mock server URL
        let client = MempoolClient::new(format!("{}/api/v1", mock_server.uri()));

        // Test hashrate endpoint
        let hashrate = client.get_hashrate(TimePeriod::ThreeMonths).await.unwrap();
        assert!(hashrate > 0.0);

        // Test block fees endpoint
        let fees = client.get_block_fees(TimePeriod::ThreeMonths).await;
        assert!(fees.unwrap() > 0.0);

        // Test difficulty adjustments endpoint
        let difficulty = client
            .get_difficulty(TimePeriod::ThreeMonths)
            .await
            .unwrap();
        assert!(difficulty > 0.0);

        // Test fee rate endpoint
        let fee_rate = client.get_fee_rate(TimePeriod::ThreeMonths).await.unwrap();
        assert!(fee_rate > 0.0);
    }
}
