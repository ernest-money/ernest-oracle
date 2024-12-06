// use crate::models::event::{Event, NewEvent};
use bitcoin::secp256k1::schnorr::Signature;
use bitcoin::secp256k1::XOnlyPublicKey;
use ddk_messages::oracle_msgs::{EventDescriptor, OracleAnnouncement};
use kormir::error::Error;
use kormir::lightning::util::ser::Readable;
use kormir::lightning::util::ser::Writeable;
use kormir::storage::OracleEventData;
use kormir::storage::Storage;
use kormir::OracleEvent;
use sqlx::{PgPool, Pool, Postgres};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct PostgresStorage {
    pool: Pool<Postgres>,
    oracle_public_key: XOnlyPublicKey,
    current_index: Arc<AtomicU32>,
}

impl PostgresStorage {
    pub async fn new(pool: PgPool, oracle_public_key: XOnlyPublicKey) -> anyhow::Result<Self> {
        let current_index =
            sqlx::query!("SELECT COALESCE(MAX(index), 0) as max_index FROM event_nonces")
                .fetch_one(&pool)
                .await?
                .max_index
                .unwrap_or(0) as u32;

        Ok(Self {
            pool,
            oracle_public_key,
            current_index: Arc::new(AtomicU32::new(current_index + 1)),
        })
    }

    pub async fn list_events(&self) -> Result<Vec<OracleEventData>, Error> {
        let mut tx = self.pool.begin().await.map_err(|_| Error::StorageFailure)?;

        let events = sqlx::query!(
            r#"
            SELECT 
                event_id, announcement_signature, oracle_event,
                announcement_event_id, attestation_event_id
            FROM events
            "#
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| Error::StorageFailure)?;

        let mut oracle_events = Vec::with_capacity(events.len());
        for event in events {
            let nonces = sqlx::query!(
                r#"
                SELECT index, outcome, signature, nonce
                FROM event_nonces
                WHERE event_id = $1
                ORDER BY index
                "#,
                event.event_id
            )
            .fetch_all(&mut *tx)
            .await
            .map_err(|_| Error::StorageFailure)?;

            let indexes = nonces.iter().map(|n| n.index as u32).collect();

            let signatures = nonces
                .into_iter()
                .filter_map(|n| {
                    if let (Some(outcome), Some(sig)) = (n.outcome, n.signature) {
                        Some((outcome, Signature::from_slice(&sig).ok()?))
                    } else {
                        None
                    }
                })
                .collect();

            let oracle_event = oracle_event(&event.oracle_event);

            let announcement = OracleAnnouncement {
                announcement_signature: Signature::from_slice(&event.announcement_signature)
                    .map_err(|_| Error::StorageFailure)?,
                oracle_public_key: self.oracle_public_key,
                oracle_event,
            };

            let data = OracleEventData {
                event_id: event.event_id,
                announcement,
                indexes,
                signatures,
            };
            oracle_events.push(data);
        }

        tx.commit().await.map_err(|_| Error::StorageFailure)?;
        Ok(oracle_events)
    }
}

impl Storage for PostgresStorage {
    async fn get_next_nonce_indexes(&self, num: usize) -> Result<Vec<u32>, Error> {
        let mut current_index = self.current_index.fetch_add(num as u32, Ordering::SeqCst);
        let mut indexes = Vec::with_capacity(num);
        for _ in 0..num {
            indexes.push(current_index);
            current_index += 1;
        }
        Ok(indexes)
    }

    async fn save_announcement(
        &self,
        announcement: OracleAnnouncement,
        indexes: Vec<u32>,
    ) -> Result<String, Error> {
        let mut tx = self.pool.begin().await.map_err(|_| Error::StorageFailure)?;

        let is_enum = matches!(
            announcement.oracle_event.event_descriptor,
            EventDescriptor::EnumEvent(_)
        );

        let event_id = announcement.oracle_event.event_id.clone();

        sqlx::query!(
            r#"
            INSERT INTO events (
                event_id, announcement_signature, oracle_event,
                name, is_enum
            )
            VALUES ($1, $2, $3, $4, $5)
            "#,
            event_id,
            announcement.announcement_signature.encode(),
            announcement.oracle_event.encode(),
            &announcement.oracle_event.event_id,
            is_enum
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| Error::StorageFailure)?;

        for (index, nonce) in indexes
            .into_iter()
            .zip(announcement.oracle_event.oracle_nonces)
        {
            sqlx::query!(
                r#"
                INSERT INTO event_nonces (
                    id, event_id, index, nonce
                )
                VALUES ($1, $2, $3, $4)
                "#,
                index as i32,
                event_id,
                index as i32,
                &nonce.serialize()
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| Error::StorageFailure)?;
        }

        tx.commit().await.map_err(|_| Error::StorageFailure)?;
        Ok(event_id)
    }

    async fn save_signatures(
        &self,
        event_id: String,
        signatures: Vec<(String, Signature)>,
    ) -> Result<OracleEventData, Error> {
        let mut tx = self.pool.begin().await.map_err(|_| Error::StorageFailure)?;

        let event = match sqlx::query!(
            r#"
            SELECT 
                event_id, announcement_signature, oracle_event,
                announcement_event_id, attestation_event_id
            FROM events
            WHERE event_id = $1
            "#,
            event_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| Error::StorageFailure)?
        {
            Some(e) => e,
            None => return Err(Error::StorageFailure),
        };

        let nonces = sqlx::query!(
            r#"
            SELECT id, index, nonce
            FROM event_nonces
            WHERE event_id = $1
            ORDER BY index
            "#,
            event_id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| Error::StorageFailure)?;

        if nonces.len() != signatures.len() {
            return Err(Error::StorageFailure);
        }

        let mut indexes = Vec::with_capacity(signatures.len());
        for (nonce, (outcome, sig)) in nonces.iter().zip(signatures.iter()) {
            sqlx::query!(
                r#"
                UPDATE event_nonces
                SET outcome = $1, signature = $2
                WHERE id = $3
                "#,
                outcome,
                sig.encode(),
                nonce.id
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| Error::StorageFailure)?;

            indexes.push(nonce.index as u32);
        }

        let oracle_event = oracle_event(&event.oracle_event);

        let data = OracleEventData {
            event_id: event.event_id.clone(),
            announcement: OracleAnnouncement {
                announcement_signature: Signature::from_slice(&event.announcement_signature)
                    .map_err(|_| Error::StorageFailure)?,
                oracle_public_key: self.oracle_public_key,
                oracle_event,
            },
            indexes,
            signatures,
        };

        tx.commit().await.map_err(|_| Error::StorageFailure)?;
        Ok(data)
    }

    async fn get_event(&self, event_id: String) -> Result<Option<OracleEventData>, Error> {
        let mut tx = self.pool.begin().await.map_err(|_| Error::StorageFailure)?;

        let event = match sqlx::query!(
            r#"
            SELECT 
                event_id, announcement_signature, oracle_event,
                announcement_event_id, attestation_event_id
            FROM events
            WHERE event_id = $1
            "#,
            event_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            log::error!("Could not retrieve event. error={}", e.to_string());
            Error::StorageFailure
        })? {
            Some(e) => e,
            None => return Ok(None),
        };

        let nonces = sqlx::query!(
            r#"
            SELECT index, outcome, signature
            FROM event_nonces
            WHERE event_id = $1
            ORDER BY index
            "#,
            event_id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| Error::StorageFailure)?;

        let indexes = nonces.iter().map(|n| n.index as u32).collect();

        let signatures = nonces
            .into_iter()
            .filter_map(|n| {
                if let (Some(outcome), Some(sig)) = (n.outcome, n.signature) {
                    Some((outcome, Signature::from_slice(&sig).ok()?))
                } else {
                    None
                }
            })
            .collect();

        let oracle_event = oracle_event(&event.oracle_event);

        let data = OracleEventData {
            event_id: event.event_id,
            announcement: OracleAnnouncement {
                announcement_signature: Signature::from_slice(&event.announcement_signature)
                    .map_err(|_| Error::StorageFailure)?,
                oracle_public_key: self.oracle_public_key,
                oracle_event,
            },
            indexes,
            signatures,
        };

        tx.commit().await.map_err(|_| Error::StorageFailure)?;
        Ok(Some(data))
    }
}

fn oracle_event(oracle_event: &Vec<u8>) -> OracleEvent {
    let mut cursor = kormir::lightning::io::Cursor::new(&oracle_event);
    OracleEvent::read(&mut cursor).expect("invalid oracle event")
}
