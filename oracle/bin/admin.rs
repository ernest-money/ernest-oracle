use std::str::FromStr;

use bitcoin::{
    key::{Keypair, Secp256k1},
    secp256k1::SecretKey,
};
use clap::Parser;
use ernest_oracle::{
    mempool::MempoolClient, oracle::ErnestOracle, parlay, storage::PostgresStorage,
};
use sqlx::PgPool;

#[derive(Debug, Clone, Parser)]
#[clap(name = "oracle-admin")]
#[clap(
    about = "CLI for the Ernest DLC Oracle",
    author = "benny b <ben@bitcoinbay.foundation>"
)]
#[clap(version = option_env ! ("CARGO_PKG_VERSION").unwrap_or("unknown"))]
struct OracleAdminArgs {
    #[clap(short, long)]
    #[clap(default_value = "postgres://loco:loco@localhost:5432/ernest-oracle")]
    db: String,
    #[clap(short, long)]
    #[clap(default_value = "34d95a073eee38ecb968a0da8273926cda601802541a715c011fb340dd6d1706")]
    key: String,
    #[clap(short, long)]
    #[clap(default_value = "https://mempool.space/api")]
    mempool: String,
    #[clap(subcommand)]
    pub command: AdminCommand,
}

#[derive(Debug, Clone, Parser)]
enum AdminCommand {
    SignEvent {
        event_id: String,
    },
    Events {
        #[clap(long)]
        id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = OracleAdminArgs::parse();
    let pool = PgPool::connect(&args.db).await?;
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_str(&args.key)?;
    let key_pair = Keypair::from_secret_key(&secp, &secret_key);
    let pubkey = key_pair.x_only_public_key();

    let storage = PostgresStorage::new(pool.clone(), pubkey.0, true).await?;
    let mempool = MempoolClient::new(args.mempool);
    let oracle = ErnestOracle::new(storage, pool.clone(), key_pair, mempool.clone())?;

    match args.command {
        AdminCommand::SignEvent { event_id } => {
            let contract = parlay::get_parlay_contract(pool, event_id.clone()).await?;
            let outcomes = contract
                .parameters
                .iter()
                .map(|parameter| {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&parameter)
                            .expect("Could not serialize parameter")
                    );
                    let outcome =
                        inquire::prompt_u64(format!("Enter outcome for {}", parameter.data_type))
                            .expect("Could not prompt for outcome") as i64;
                    let normalized_value = parameter.normalize_parameter(outcome);
                    println!(
                        "normalized value for {:?}: {:?}",
                        parameter.data_type, normalized_value
                    );
                    let transformed_value = parameter.apply_transformation(normalized_value);
                    println!(
                        "transformed value for {:?}: {:?}",
                        parameter.data_type, transformed_value
                    );
                    transformed_value
                })
                .collect::<Vec<_>>();
            let combined_score =
                parlay::combine_scores(&outcomes, &[], &contract.combination_method);
            println!(
                "combined score for contract {:?}: {:?}",
                contract.id, combined_score
            );
            let attestable_value =
                parlay::convert_to_attestable_value(combined_score, contract.max_normalized_value);
            println!(
                "attestable value for contract {:?}: {:?}",
                contract.id, attestable_value
            );
            oracle
                .oracle
                .sign_numeric_event(event_id.clone(), attestable_value as i64)
                .await?;
            println!("Signed event {:?}", event_id);
        }
        AdminCommand::Events { id } => {
            let events = oracle.oracle.storage.list_events().await?;
            if let Some(id) = id {
                let event = events
                    .iter()
                    .find(|e| e.announcement.oracle_event.event_id == id);
                if let Some(event) = event {
                    print!("{}", serde_json::to_string_pretty(event)?);
                } else {
                    println!("Event not found");
                }
            } else {
                print!("{}", serde_json::to_string_pretty(&events)?);
            }
        }
    }
    Ok(())
}
