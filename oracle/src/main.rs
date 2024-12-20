#![allow(dead_code)]
mod events;
mod mempool;
mod oracle;
mod routes;
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
};
use kormir::{storage::OracleEventData, OracleAnnouncement, OracleAttestation};
use log::LevelFilter;
use mempool::{MempoolClient, BASE_URL};
use oracle::ErnestOracle;
use routes::{CreateEvent, GetAnnouncement, GetAttestation, SignEvent};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{str::FromStr, sync::Arc};
use storage::PostgresStorage;

pub const IS_SIGNED: bool = false;
pub const PRECISION: i32 = 2;

#[derive(Debug, Serialize, Deserialize)]
struct OracleError {
    pub reason: String,
}

struct OracleState {
    pub oracle: ErnestOracle,
    pub mempool: MempoolClient,
}

async fn hello() -> Html<&'static str> {
    Html("<h1 style='width: 100%; height: 100vh; display: flex; justify-content: center; align-items: center; font-family: sans-serif; margin: 0;'>Ernest Oracle</h1>")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();
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
    let mempool = MempoolClient::new(BASE_URL.to_string());

    let state = Arc::new(OracleState { oracle, mempool });

    let state_clone = state.clone();
    tokio::spawn(async move {
        watcher::sign_matured_events_loop(state_clone).await;
    });

    let app = Router::new()
        .route("/", get(hello))
        .route("/info", get(oracle_info))
        .route("/list-events", get(list_events))
        .route("/create-event", post(create_event))
        .route("/announcement", get(get_announcement_event))
        .route("/attestation", get(get_attestation))
        .route("/sign-event", post(sign_event))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();

    log::info!("Serving hashrate oracle");

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn create_event(
    State(state): State<Arc<OracleState>>,
    Json(event): Json<CreateEvent>,
) -> Result<Json<OracleAnnouncement>, (StatusCode, Json<OracleError>)> {
    match routes::create_event_internal(state, event).await {
        Ok(event) => Ok(Json(event)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: e.to_string(),
            }),
        )),
    }
}

async fn get_announcement_event(
    State(state): State<Arc<OracleState>>,
    event: Query<GetAnnouncement>,
) -> Result<Json<OracleAnnouncement>, (StatusCode, Json<OracleError>)> {
    match routes::get_announcement_internal(state, event.0).await {
        Ok(event) => Ok(Json(event)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: e.reason.to_string(),
            }),
        )),
    }
}

async fn get_attestation(
    State(state): State<Arc<OracleState>>,
    event: Query<GetAttestation>,
) -> Result<Json<OracleAttestation>, (StatusCode, Json<OracleError>)> {
    match routes::get_attestation_internal(state, event.0).await {
        Ok(attestation) => Ok(Json(attestation)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: e.to_string(),
            }),
        )),
    }
}

async fn sign_event(
    State(state): State<Arc<OracleState>>,
    Json(event): Json<SignEvent>,
) -> Result<Json<OracleAttestation>, (StatusCode, Json<OracleError>)> {
    match routes::sign_event_internal(state, event).await {
        Ok(attestation) => Ok(Json(attestation)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: e.to_string(),
            }),
        )),
    }
}

async fn oracle_info(State(state): State<Arc<OracleState>>) -> impl IntoResponse {
    Json(routes::oracle_info_internal(state).await).into_response()
}

async fn list_events(
    State(state): State<Arc<OracleState>>,
) -> Result<Json<Vec<OracleEventData>>, (StatusCode, Json<OracleError>)> {
    match routes::list_events_internal(state).await {
        Ok(events) => Ok(Json(events)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleError {
                reason: e.to_string(),
            }),
        )),
    }
}
