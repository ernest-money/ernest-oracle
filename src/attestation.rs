use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool, Postgres};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErnestOracleOutcome {
    pub event_id: String,
    pub combined_score: f64,
    pub attested_value: i32,
    pub outcomes: Vec<AttestationDataOutcome>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AttestationOutcome {
    pub event_id: String,
    pub combined_score: f64,
    pub attested_value: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AttestationDataOutcome {
    pub event_id: String,
    pub data_type: String,
    pub normalized_value: f64,
    pub original_value: f64,
}

pub async fn get_attestation_outcome(
    pool: &PgPool,
    event_id: String,
) -> anyhow::Result<ErnestOracleOutcome> {
    let outcome = sqlx::query_as::<Postgres, AttestationOutcome>(
        "SELECT * FROM numeric_attestation_outcome WHERE event_id = $1",
    )
    .bind(&event_id)
    .fetch_one(&*pool)
    .await?;

    let data_outcomes = sqlx::query_as::<Postgres, AttestationDataOutcome>(
        "SELECT * FROM numeric_attestation_data_outcome WHERE event_id = $1",
    )
    .bind(&event_id)
    .fetch_all(&*pool)
    .await?;

    let outcomes = data_outcomes
        .into_iter()
        .map(|outcome| AttestationDataOutcome {
            event_id: outcome.event_id,
            data_type: outcome.data_type,
            normalized_value: outcome.normalized_value,
            original_value: outcome.original_value,
        })
        .collect();

    Ok(ErnestOracleOutcome {
        event_id,
        combined_score: outcome.combined_score,
        attested_value: outcome.attested_value,
        outcomes,
    })
}

pub async fn save_attestation_data_outcomes(
    pool: &PgPool,
    outcomes: Vec<AttestationDataOutcome>,
) -> anyhow::Result<()> {
    for outcome in outcomes {
        save_attestation_data_outcome(
            pool,
            outcome.event_id,
            outcome.data_type,
            outcome.normalized_value,
            outcome.original_value,
        )
        .await?;
    }
    Ok(())
}

pub async fn save_attestation_data_outcome(
    pool: &PgPool,
    event_id: String,
    data_type: String,
    normalized_value: f64,
    original_value: f64,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
      "INSERT INTO numeric_attestation_data_outcome (event_id, data_type, normalized_value, original_value) VALUES ($1, $2, $3, $4)",
    )
    .bind(&event_id)
    .bind(&data_type)
    .bind(&normalized_value)
    .bind(&original_value)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn save_attestation_outcome(
    pool: &PgPool,
    event_id: String,
    combined_score: f64,
    attested_value: u64,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO numeric_attestation_outcome (event_id, combined_score, attested_value) VALUES ($1, $2, $3)",
    )
    .bind(&event_id)
    .bind(&combined_score)
    .bind(attested_value as i64)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
