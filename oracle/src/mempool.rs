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

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockReward {
    #[serde(rename = "avgHeight")]
    pub avg_height: i64,
    pub timestamp: i64,
    #[serde(rename = "avgRewards")]
    pub avg_rewards: i64,
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
pub struct BlockFees {
    #[serde(rename = "avgHeight")]
    pub avg_height: i64,
    pub timestamp: i64,
    #[serde(rename = "avgFees")]
    pub avg_fees: i64,
}

pub struct MempoolClient {
    client: Client,
    base_url: String,
}

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
            _ => format!("{}/mining/hashrate/{}", BASE_URL, period.as_str()),
        };

        let response = self.client.get(&url).send().await?;
        let data = response.json::<HashrateResponse>().await?;
        let average_hashrate = Self::calculate_average(data.hashrates, |h| h.avg_hashrate);
        Ok(average_hashrate)
    }

    pub async fn get_block_rewards(&self, period: TimePeriod) -> anyhow::Result<f64> {
        let url = format!(
            "{}/mining/blocks/rewards/{}",
            self.base_url,
            period.as_str()
        );
        let response = self.client.get(&url).send().await?;
        let data = response.json::<Vec<BlockReward>>().await?;
        let average_rewards = Self::calculate_average(data, |r| r.avg_rewards as f64);
        Ok(average_rewards)
    }

    pub async fn get_difficulty_adjustments(&self, interval: TimePeriod) -> anyhow::Result<f64> {
        let url = format!(
            "{}/mining/difficulty-adjustments/{}",
            self.base_url,
            interval.as_str()
        );

        let response = self.client.get(&url).send().await?;
        let data = response.json::<Vec<DifficultyAdjustment>>().await?;
        let average_difficulty = Self::calculate_average(data, |d| d.difficulty);
        Ok(average_difficulty)
    }

    pub async fn get_block_fees(&self, period: TimePeriod) -> anyhow::Result<f64> {
        let url = format!("{}/mining/blocks/fees/{}", self.base_url, period.as_str());
        let response = self.client.get(&url).send().await?;
        let data = response.json::<Vec<BlockFees>>().await?;
        let average_fees = Self::calculate_average(data, |f| f.avg_fees as f64);
        Ok(average_fees)
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
    use super::MempoolClient; // Your actual client
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    pub async fn setup_mock_server() -> MockServer {
        let mock_server = MockServer::start().await;

        // Mock hashrate endpoint
        Mock::given(method("GET"))
            .and(path("/api/v1/mining/hashrate/3m"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "hashrates": [
                    {
                        "timestamp": 1652486400,
                        "avgHashrate": 2364997621087718_i64
                    }
                ],
                "difficulty": [
                    {
                        "timestamp": 1652468330,
                        "difficulty": 31251101365711.12,
                        "height": 736249
                    }
                ],
                "currentHashrate": 2520332473552123_i64,
                "currentDifficulty": 31251101365711.12
            })))
            .mount(&mock_server)
            .await;

        // Mock block fees endpoint
        Mock::given(method("GET"))
            .and(path("/api/v1/mining/blocks/fees/3m"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "avgHeight": 735644,
                    "timestamp": 1652119111,
                    "avgFees": 24212890
                }
            ])))
            .mount(&mock_server)
            .await;

        // Mock block rewards endpoint
        Mock::given(method("GET"))
            .and(path("/api/v1/mining/blocks/rewards/3m"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "avgHeight": 599992,
                    "timestamp": 1571438412,
                    "avgRewards": 1260530933
                }
            ])))
            .mount(&mock_server)
            .await;

        // Mock difficulty adjustments endpoint
        Mock::given(method("GET"))
            .and(path("/api/v1/mining/difficulty-adjustments/3m"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([[
                1703311464,
                822528,
                72006146478567.1,
                1.06983
            ]])))
            .mount(&mock_server)
            .await;

        mock_server
    }

    #[tokio::test]
    async fn test_mempool_client_with_mock_server() {
        // Start mock server
        let mock_server = setup_mock_server().await;

        // Create client with mock server URL
        let client = MempoolClient::new(format!("{}/api/v1", mock_server.uri()));

        // Test hashrate endpoint
        let hashrate = client.get_hashrate(TimePeriod::ThreeMonths).await.unwrap();
        println!("{:?}", hashrate);
        assert!(hashrate > 0.0);

        // Test block fees endpoint
        let fees = client.get_block_fees(TimePeriod::ThreeMonths).await;
        println!("{:?}", fees);
        assert!(fees.unwrap() > 0.0);

        // Test block rewards endpoint
        let rewards = client
            .get_block_rewards(TimePeriod::ThreeMonths)
            .await
            .unwrap();
        assert!(rewards > 0.0);

        // Test difficulty adjustments endpoint
        let adjustments = client
            .get_difficulty_adjustments(TimePeriod::ThreeMonths)
            .await
            .unwrap();
        assert!(adjustments > 0.0);
    }
}
