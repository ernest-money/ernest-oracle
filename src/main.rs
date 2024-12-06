#![allow(dead_code)]
mod mempool;
mod oracle;
mod storage;
mod watcher;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use bitcoin::{
    key::{Keypair, Secp256k1},
    secp256k1::SecretKey,
    XOnlyPublicKey,
};
use kormir::{
    storage::{OracleEventData, Storage},
    OracleAnnouncement, OracleAttestation,
};
use mempool::MempoolClient;
use oracle::ErnestOracle;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{str::FromStr, sync::Arc};
use storage::PostgresStorage;
use uuid::Uuid;

const NB_DIGITS: u16 = 30;
const UNIT: &str = "H/s";

#[derive(Debug, Serialize, Deserialize)]
struct OracleError {
    reason: String,
}

struct OracleState {
    pub oracle: ErnestOracle,
    pub mempool: MempoolClient,
}

async fn hello() -> Html<&'static str> {
    Html("<h1>Ernest Oracle</h1>")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    log::info!("Starting Ernest Hashrate Oracle");

    let pg_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&pg_url).await?;
    let secp = Secp256k1::new();
    let kormir_key = std::env::var("ERNEST_KEY")?;
    let secret_key = SecretKey::from_str(&kormir_key)?;
    let key_pair = Keypair::from_secret_key(&secp, &secret_key);
    let pubkey = key_pair.x_only_public_key();

    let storage = PostgresStorage::new(pool, pubkey.0).await?;
    let oracle = ErnestOracle::new(storage, key_pair)?;
    let mempool = MempoolClient::new();

    let state = Arc::new(OracleState { oracle, mempool });

    let state_clone = state.clone();
    tokio::spawn(async move {
        watcher::sign_matured_events(state_clone).await;
    });

    let app = Router::new()
        .route("/", get(hello))
        .route("/info", get(oracle_info))
        .route("/list-events", get(list_events))
        .route("/create-event", post(create_event))
        .route("/event", get(event))
        .route("/sign-event", post(sign_event))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();

    log::info!("Serving hashrate oracle");

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEvent {
    maturity: u32,
}

async fn create_event(
    State(state): State<Arc<OracleState>>,
    Json(event): Json<CreateEvent>,
) -> Result<Json<OracleAnnouncement>, (StatusCode, Json<OracleError>)> {
    let event_id = Uuid::new_v4().to_string();
    match state
        .oracle
        .oracle
        .create_numeric_event(
            event_id,
            NB_DIGITS,
            false,
            2,
            UNIT.to_string(),
            event.maturity,
        )
        .await
    {
        Ok(event) => Ok(Json(event)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: e.to_string(),
            }),
        )),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetEvent {
    event_id: String,
}

async fn event(
    State(state): State<Arc<OracleState>>,
    event: Query<GetEvent>,
) -> Result<Json<OracleAnnouncement>, (StatusCode, Json<OracleError>)> {
    match state
        .oracle
        .oracle
        .storage
        .get_event(event.0.event_id)
        .await
    {
        Ok(event) => match event {
            Some(e) => Ok(Json(e.announcement)),
            None => Err((
                StatusCode::NOT_FOUND,
                Json(OracleError {
                    reason: "Oracle event not found".to_string(),
                }),
            )),
        },
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: format!("Failed to retrieve oracle event. error={}", e.to_string()),
            }),
        )),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SignEvent {
    event_id: String,
}

async fn sign_event(
    State(state): State<Arc<OracleState>>,
    Json(event): Json<SignEvent>,
) -> Result<Json<OracleAttestation>, (StatusCode, Json<OracleError>)> {
    let hashrate = state
        .mempool
        .get_hashrate(mempool::TimePeriod::ThreeMonths)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(OracleError {
                    reason: format!(
                        "Could not get hashrate from mempool.space error={}",
                        e.to_string()
                    ),
                }),
            )
        })?;

    match state
        .oracle
        .oracle
        .sign_numeric_event(event.event_id, hashrate)
        .await
    {
        Ok(event) => Ok(Json(event)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: e.to_string(),
            }),
        )),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OracleInfo {
    pubkey: XOnlyPublicKey,
    name: String,
}

async fn oracle_info(State(state): State<Arc<OracleState>>) -> impl IntoResponse {
    let pubkey = state.oracle.oracle.public_key();
    let oracle_info = OracleInfo {
        pubkey,
        name: "Ernest Hashrate Oracle".to_string(),
    };
    Json(oracle_info).into_response()
}

async fn list_events(
    State(state): State<Arc<OracleState>>,
) -> Result<Json<Vec<OracleEventData>>, (StatusCode, Json<OracleError>)> {
    let events = state
        .oracle
        .oracle
        .storage
        .list_events()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(OracleError {
                    reason: e.to_string(),
                }),
            )
        })?;

    Ok(Json(events))
}
