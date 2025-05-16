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
use ernest_oracle::oracle::ErnestOracle;
use ernest_oracle::routes;
use ernest_oracle::storage::PostgresStorage;
use ernest_oracle::{
    mempool::{MempoolClient, BASE_URL},
    parlay::ParlayContract,
};
use ernest_oracle::{OracleServerError, OracleServerState};
use kormir::{storage::OracleEventData, OracleAnnouncement, OracleAttestation};
use log::LevelFilter;
use sqlx::PgPool;
use std::{str::FromStr, sync::Arc};

pub const PORT: u16 = 3001;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();
    log::info!("Starting Ernest Hashrate Oracle");

    let port = std::env::var("PORT").unwrap_or(PORT.to_string());

    let pg_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&pg_url).await?;
    let secp = Secp256k1::new();
    let kormir_key = std::env::var("ERNEST_KEY")?;
    let secret_key = SecretKey::from_str(&kormir_key)?;
    let key_pair = Keypair::from_secret_key(&secp, &secret_key);
    let pubkey = key_pair.x_only_public_key();

    let storage = PostgresStorage::new(pool.clone(), pubkey.0, true).await?;
    let mempool = MempoolClient::new(BASE_URL.to_string());
    let oracle = ErnestOracle::new(storage, pool, key_pair, mempool.clone())?;

    let state = Arc::new(OracleServerState { oracle, mempool });

    let _state_clone = state.clone();
    // tokio::spawn(async move {
    //     ernest_oracle::watcher::sign_matured_events_loop(state_clone).await;
    // });

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/", get(hello))
                .route("/info", get(oracle_info))
                .route("/list-events", get(list_events))
                .route("/create", post(create_event))
                .route("/announcement", get(get_announcement_event))
                .route("/attestation", get(get_attestation))
                .route("/sign-event", post(sign_event))
                .route("/parlay", get(get_parlay_contract)),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    log::info!("Serving hashrate oracle on port {}", port);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn hello() -> Html<&'static str> {
    Html("<h1 style='width: 100%; height: 100vh; display: flex; justify-content: center; align-items: center; font-family: sans-serif; margin: 0;'>Ernest Oracle</h1>")
}

#[axum::debug_handler]
async fn create_event(
    State(state): State<Arc<OracleServerState>>,
    Json(event): Json<routes::CreateEvent>,
) -> Result<Json<OracleAnnouncement>, (StatusCode, Json<OracleServerError>)> {
    log::info!("Creating event {:?}", event);
    match routes::create_event_internal(state, event).await {
        Ok(event) => Ok(Json(event)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleServerError {
                reason: e.to_string(),
            }),
        )),
    }
}

async fn get_announcement_event(
    State(state): State<Arc<OracleServerState>>,
    event: Query<routes::GetAnnouncement>,
) -> Result<Json<OracleAnnouncement>, (StatusCode, Json<OracleServerError>)> {
    match routes::get_announcement_internal(state, event.0).await {
        Ok(event) => Ok(Json(event)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleServerError {
                reason: e.reason.to_string(),
            }),
        )),
    }
}

async fn get_attestation(
    State(state): State<Arc<OracleServerState>>,
    event: Query<routes::GetAttestation>,
) -> Result<Json<OracleAttestation>, (StatusCode, Json<OracleServerError>)> {
    match routes::get_attestation_internal(state, event.0).await {
        Ok(attestation) => Ok(Json(attestation)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleServerError {
                reason: e.to_string(),
            }),
        )),
    }
}

async fn sign_event(
    State(state): State<Arc<OracleServerState>>,
    Json(event): Json<routes::SignEvent>,
) -> Result<Json<OracleAttestation>, (StatusCode, Json<OracleServerError>)> {
    match routes::sign_event_internal(state, event).await {
        Ok(attestation) => Ok(Json(attestation)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleServerError {
                reason: e.to_string(),
            }),
        )),
    }
}

async fn oracle_info(State(state): State<Arc<OracleServerState>>) -> impl IntoResponse {
    Json(routes::oracle_info_internal(state).await).into_response()
}

async fn list_events(
    State(state): State<Arc<OracleServerState>>,
) -> Result<Json<Vec<OracleEventData>>, (StatusCode, Json<OracleServerError>)> {
    match routes::list_events_internal(state).await {
        Ok(events) => Ok(Json(events)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleServerError {
                reason: e.to_string(),
            }),
        )),
    }
}

async fn get_parlay_contract(
    State(state): State<Arc<OracleServerState>>,
    event: Query<routes::GetParlayContract>,
) -> Result<Json<ParlayContract>, (StatusCode, Json<OracleServerError>)> {
    match routes::get_parlay_contract_internal(state, event.0).await {
        Ok(event) => Ok(Json(event)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(OracleServerError {
                reason: e.to_string(),
            }),
        )),
    }
}
