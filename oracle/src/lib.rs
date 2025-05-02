#![allow(dead_code)]
mod events;
pub mod mempool;
pub mod oracle;
pub mod parlay;
pub mod routes;
pub mod storage;
mod test_util;
pub mod watcher;

use bitcoin::XOnlyPublicKey;
use ddk::Oracle;
use ddk_manager::Oracle as DlcOracle;
use dlc_messages::oracle_msgs::{OracleAnnouncement, OracleAttestation};
use kormir::storage::OracleEventData;
use parlay::ParlayContract;
use reqwest::Client;
use routes::{CreateEvent, OracleInfo, SignEvent};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OracleServerError {
    pub reason: String,
}

pub struct OracleServerState {
    pub oracle: oracle::ErnestOracle,
    pub mempool: mempool::MempoolClient,
}

pub fn oracle_err_to_manager_err(e: OracleServerError) -> ddk_manager::error::Error {
    ddk_manager::error::Error::OracleError(e.reason.to_string())
}

pub struct ErnestOracleClient {
    client: Client,
    base_url: String,
    pubkey: XOnlyPublicKey,
}

impl ErnestOracleClient {
    pub async fn new(base_url: &str) -> Result<ErnestOracleClient, OracleServerError> {
        let client = Client::new();
        let info = client
            .get(format!("{}/api/info", &base_url))
            .send()
            .await
            .map_err(|e| OracleServerError {
                reason: e.to_string(),
            })?
            .json::<OracleInfo>()
            .await
            .map_err(|e| OracleServerError {
                reason: e.to_string(),
            })?;

        Ok(ErnestOracleClient {
            client,
            base_url: base_url.to_string(),
            pubkey: info.pubkey,
        })
    }
    async fn get<T>(&self, path: &str) -> Result<T, OracleServerError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        println!("url: {}", url);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| OracleServerError {
                reason: e.to_string(),
            })?
            .json::<T>()
            .await
            .map_err(|_| OracleServerError {
                reason: "Couldn't serde parse type.".to_string(),
            })?;
        Ok(response)
    }
    pub async fn create_event(
        &self,
        event: CreateEvent,
    ) -> Result<OracleAnnouncement, reqwest::Error> {
        let url = format!("{}/api/create", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&event)
            .send()
            .await?
            .json::<OracleAnnouncement>()
            .await?;
        Ok(response)
    }

    pub async fn get_announcement_event(
        &self,
        event_id: &str,
    ) -> Result<OracleAnnouncement, OracleServerError> {
        let path = format!("/api/announcement?event_id={}", event_id);
        let response = self.get::<OracleAnnouncement>(&path).await?;
        Ok(response)
    }

    pub async fn get_attestation_event(
        &self,
        event_id: &str,
    ) -> Result<OracleAttestation, OracleServerError> {
        let path = format!("/api/attestation?event_id={}", event_id);
        let response = self.get::<OracleAttestation>(&path).await?;
        Ok(response)
    }

    pub async fn get_parlay_contract(
        &self,
        event_id: &str,
    ) -> Result<ParlayContract, OracleServerError> {
        let path = format!("/api/parlay?event_id={}", event_id);
        let response = self.get::<ParlayContract>(&path).await?;
        Ok(response)
    }
    async fn sign_event(&self, event: SignEvent) -> Result<OracleAttestation, OracleServerError> {
        let url = format!("{}/api/sign-event", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&event)
            .send()
            .await
            .map_err(|e| OracleServerError {
                reason: e.to_string(),
            })?
            .json::<OracleAttestation>()
            .await
            .map_err(|e| OracleServerError {
                reason: e.to_string(),
            })?;
        Ok(response)
    }

    pub async fn get_oracle_info(&self) -> Result<OracleInfo, OracleServerError> {
        let response = self.get::<OracleInfo>("/api/info").await?;
        Ok(response)
    }

    pub async fn list_events(&self) -> Result<Vec<OracleEventData>, OracleServerError> {
        let events = self.get::<Vec<OracleEventData>>("/api/list-events").await?;
        Ok(events)
    }
}

impl Oracle for ErnestOracleClient {
    fn name(&self) -> String {
        "Ernest Oracle".to_string()
    }
}

#[async_trait::async_trait]
impl DlcOracle for ErnestOracleClient {
    /// Returns the public key of the oracle.
    fn get_public_key(&self) -> XOnlyPublicKey {
        self.pubkey
    }
    /// Returns the announcement for the event with the given id if found.
    async fn get_announcement(
        &self,
        event_id: &str,
    ) -> Result<OracleAnnouncement, ddk_manager::error::Error> {
        self.get_announcement_event(event_id)
            .await
            .map_err(oracle_err_to_manager_err)
    }
    /// Returns the attestation for the event with the given id if found.
    async fn get_attestation(
        &self,
        event_id: &str,
    ) -> Result<OracleAttestation, ddk_manager::error::Error> {
        self.get_attestation_event(event_id)
            .await
            .map_err(oracle_err_to_manager_err)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        events::EventType,
        parlay::{CombinationMethod, ParlayParameter, TransformationFunction},
    };

    use super::*;

    async fn create_event(client: &ErnestOracleClient) -> (OracleAnnouncement, CreateEvent) {
        let now = Utc::now().timestamp();
        let event = CreateEvent::Parlay {
            parameters: vec![
                ParlayParameter {
                    data_type: EventType::Hashrate,
                    threshold: 5000,
                    range: 100000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.0,
                },
                ParlayParameter {
                    data_type: EventType::DificultyAdjustment,
                    threshold: 150000,
                    range: 1000000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.0,
                },
            ],
            combination_method: CombinationMethod::Multiply,
            max_normalized_value: Some(10000),
            event_maturity_epoch: (now + 1000) as u32,
        };
        let announcement = client.create_event(event.clone()).await.unwrap();
        (announcement, event)
    }

    #[tokio::test]
    async fn oracle_info() {
        let oracle_url = std::env::var("ORACLE_URL").expect("ORACLE_URL must be set");
        let client = ErnestOracleClient::new(&oracle_url).await.unwrap();
        let info = client.get_oracle_info().await;
        assert!(info.is_ok())
    }

    #[tokio::test]
    async fn test_oracle_client() {
        let oracle_url = std::env::var("ORACLE_URL").expect("ORACLE_URL must be set");
        let client = ErnestOracleClient::new(&oracle_url).await.unwrap();
        let (announcement, event) = create_event(&client).await;
        let events = client.list_events().await.unwrap();
        assert!(events.len() > 0);

        let oracle_announcement = client
            .get_announcement_event(&announcement.oracle_event.event_id)
            .await
            .unwrap();
        assert_eq!(
            announcement.oracle_event.event_id,
            oracle_announcement.oracle_event.event_id
        );

        let oracle_parlay_contract = client
            .get_parlay_contract(&announcement.oracle_event.event_id)
            .await
            .unwrap();

        let parlay_contract = if let CreateEvent::Parlay {
            parameters,
            combination_method,
            max_normalized_value,
            event_maturity_epoch: _,
        } = event
        {
            ParlayContract {
                id: announcement.oracle_event.event_id,
                parameters,
                combination_method,
                max_normalized_value: max_normalized_value.unwrap(),
            }
        } else {
            panic!("Event is not a parlay");
        };
        assert_eq!(oracle_parlay_contract, parlay_contract);
    }

    #[tokio::test]
    async fn test_oracle_client_multiple_events() {
        let oracle_url = std::env::var("ORACLE_URL").expect("ORACLE_URL must be set");
        let client = ErnestOracleClient::new(&oracle_url).await.unwrap();
        let now = Utc::now().timestamp();
        let event = CreateEvent::Parlay {
            parameters: vec![
                ParlayParameter {
                    data_type: EventType::Hashrate,
                    threshold: 5000,
                    range: 100000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.0,
                },
                ParlayParameter {
                    data_type: EventType::DificultyAdjustment,
                    threshold: 150000,
                    range: 1000000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.0,
                },
            ],
            combination_method: CombinationMethod::Multiply,
            max_normalized_value: Some(10000),
            event_maturity_epoch: (now + 1000) as u32,
        };

        let now = Utc::now().timestamp();
        let event_two = CreateEvent::Parlay {
            parameters: vec![
                ParlayParameter {
                    data_type: EventType::Hashrate,
                    threshold: 5000,
                    range: 100000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.0,
                },
                ParlayParameter {
                    data_type: EventType::DificultyAdjustment,
                    threshold: 150000,
                    range: 1000000,
                    is_above_threshold: true,
                    transformation: TransformationFunction::Linear,
                    weight: 1.0,
                },
            ],
            combination_method: CombinationMethod::Multiply,
            max_normalized_value: Some(10000),
            event_maturity_epoch: (now + 1000) as u32,
        };
        client.create_event(event.clone()).await.unwrap();
        client.create_event(event_two.clone()).await.unwrap();
    }
}
