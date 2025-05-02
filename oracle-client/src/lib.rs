mod types;

use bitcoin::XOnlyPublicKey;
use ddk::Oracle;
use ddk_manager::Oracle as DlcOracle;
use dlc_messages::oracle_msgs::{OracleAnnouncement, OracleAttestation};
use kormir::storage::OracleEventData;
use reqwest::Client;
use types::{OracleError, OracleInfo, SignEvent};

pub struct ErnestOracle {
    client: Client,
    base_url: String,
    pubkey: XOnlyPublicKey,
}

impl ErnestOracle {
    pub async fn new(base_url: &str) -> Result<ErnestOracle, OracleError> {
        let client = Client::new();
        let info = client
            .get(format!("{}/info", &base_url))
            .send()
            .await
            .map_err(|e| OracleError {
                reason: e.to_string(),
            })?
            .json::<OracleInfo>()
            .await
            .map_err(|e| OracleError {
                reason: e.to_string(),
            })?;

        Ok(ErnestOracle {
            client,
            base_url: base_url.to_string(),
            pubkey: info.pubkey,
        })
    }
    async fn get<T>(&self, path: &str) -> Result<T, OracleError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, path);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| OracleError {
                reason: e.to_string(),
            })?
            .json::<T>()
            .await
            .map_err(|_| OracleError {
                reason: "Couldn't serde parse type.".to_string(),
            })?;
        Ok(response)
    }
    pub async fn create_event(
        &self,
        event: crate::types::CreateEvent,
    ) -> Result<OracleAnnouncement, reqwest::Error> {
        let url = format!("{}/event", self.base_url);
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
    ) -> Result<OracleAnnouncement, OracleError> {
        let path = format!("announcement/{}", event_id);
        let response = self.get::<OracleAnnouncement>(&path).await?;
        Ok(response)
    }

    pub async fn get_attestation_event(
        &self,
        event_id: &str,
    ) -> Result<OracleAttestation, OracleError> {
        let path = format!("attestation/{}", event_id);
        let response = self.get::<OracleAttestation>(&path).await?;
        Ok(response)
    }

    pub async fn sign_event(&self, event: SignEvent) -> Result<OracleAttestation, OracleError> {
        let url = format!("{}/event/{}/sign", self.base_url, event.event_id);
        let response = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| OracleError {
                reason: e.to_string(),
            })?
            .json::<OracleAttestation>()
            .await
            .map_err(|e| OracleError {
                reason: e.to_string(),
            })?;
        Ok(response)
    }

    pub async fn get_oracle_info(&self) -> Result<OracleInfo, OracleError> {
        let response = self.get::<OracleInfo>("info").await?;
        Ok(response)
    }

    pub async fn list_events(&self) -> Result<Vec<OracleEventData>, OracleError> {
        self.get("events").await?
    }
}

impl Oracle for ErnestOracle {
    fn name(&self) -> String {
        "Ernest Oracle".to_string()
    }
}

#[async_trait::async_trait]
impl DlcOracle for ErnestOracle {
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

pub fn oracle_err_to_manager_err(e: OracleError) -> ddk_manager::error::Error {
    ddk_manager::error::Error::OracleError(e.reason.to_string())
}
