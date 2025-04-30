#![allow(dead_code)]
mod events;
pub mod mempool;
pub mod oracle;
mod parlay;
pub mod routes;
pub mod storage;
mod test_util;
pub mod watcher;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OracleError {
    pub reason: String,
}

pub struct OracleState {
    pub oracle: oracle::ErnestOracle,
    pub mempool: mempool::MempoolClient,
}
