use crate::mempool::MempoolClient;
use crate::oracle::ErnestOracle;
use crate::parlay::parameter::ParlayParameter;
use crate::storage::PostgresStorage;
use bitcoin::key::{Keypair, Secp256k1};
use bitcoin::secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub async fn setup_ernest_oracle(mempool: MempoolClient) -> ErnestOracle {
    let pg_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let pool = PgPool::connect(&pg_url)
        .await
        .expect("Failed to connect to database");
    let secp = Secp256k1::new();
    let kormir_key = std::env::var("ERNEST_KEY").expect("ERNEST_KEY is not set");
    let secret_key = SecretKey::from_str(&kormir_key).expect("Failed to parse ERNEST_KEY");
    let key_pair = Keypair::from_secret_key(&secp, &secret_key);
    let pubkey = key_pair.x_only_public_key();

    let storage = PostgresStorage::new(pool.clone(), pubkey.0, true)
        .await
        .expect("Failed to create PostgresStorage");
    ErnestOracle::new(storage, pool, key_pair, mempool).expect("Failed to create ErnestOracle")
}

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
                    "time": 1652468330,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TestVectors {
    pub test_vectors: Vec<TestVector>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestVector {
    pub name: String,
    pub contract: Contract,
    pub mock_inputs: HashMap<String, i64>,
    pub expected: Expected,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contract {
    pub id: String,
    pub parameters: Vec<ParlayParameter>,
    pub combination_method: String,
    pub max_normalized_value: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Expected {
    pub normalized_values: Vec<f64>,
    pub transformed_values: Vec<f64>,
    pub combined_score: f64,
    pub attestation_value: u64,
}

pub async fn setup_mock_server_from_test_vectors(test_vector: TestVector) -> MockServer {
    // Read the test vectors file
    let mock_server = MockServer::start().await;

    // Get the first test vector for simplicity
    // You could extend this to support multiple test vectors with more complex logic

    for (data_type, value) in &test_vector.mock_inputs {
        match data_type.as_str() {
            "hashrate" => {
                Mock::given(method("GET"))
                    .and(path("/api/v1/mining/hashrate"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                        "hashrates": [
                            {
                                "timestamp": 1652486400,
                                "avgHashrate": value
                            }
                        ],
                        "difficulty": [
                            {
                                "time": 1652468330,
                                "difficulty": 31251101365711.12,
                                "adjustment": 1.06983,
                                "height": 736249
                            }
                        ],
                        "currentHashrate": value,
                        "currentDifficulty": 31251101365711.12
                    })))
                    .mount(&mock_server)
                    .await;
            }
            "block-fees" => {
                Mock::given(method("GET"))
                    .and(path("/api/v1/mining/blocks/fees/3m"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                        {
                            "avgHeight": 735644,
                            "timestamp": 1652119111,
                            "avgFees": value
                        }
                    ])))
                    .mount(&mock_server)
                    .await;
            }
            "block-rewards" => {
                Mock::given(method("GET"))
                    .and(path("/api/v1/mining/blocks/rewards/3m"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                        {
                            "avgHeight": 599992,
                            "timestamp": 1571438412,
                            "avgRewards": value
                        }
                    ])))
                    .mount(&mock_server)
                    .await;
            }
            "difficulty-adjustments" => {
                Mock::given(method("GET"))
                    .and(path("/api/v1/mining/difficulty-adjustments/3m"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_json(json!([[1703311464, 822528, value, 1.06983]])),
                    )
                    .mount(&mock_server)
                    .await;
            }
            _ => {
                // For any other data types, create a generic endpoint
                Mock::given(method("GET"))
                    .and(path("/test/mock"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                        "success": true
                    })))
                    .mount(&mock_server)
                    .await;
            }
        }
    }

    mock_server
}
