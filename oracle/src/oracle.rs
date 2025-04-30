use crate::{events::EventType, mempool::MempoolClient, parlay, storage::PostgresStorage};
use bitcoin::{
    bip32::Xpriv,
    key::{Keypair, Secp256k1},
    secp256k1::All,
    Network, XOnlyPublicKey,
};
use kormir::Oracle;
use sqlx::PgPool;

pub struct ErnestOracle {
    pub oracle: Oracle<PostgresStorage>,
    pubkey: XOnlyPublicKey,
    mempool: MempoolClient,
    secp: Secp256k1<All>,
    pool: PgPool,
}

impl ErnestOracle {
    pub fn new(
        storage: PostgresStorage,
        pool: PgPool,
        keypair: Keypair,
        mempool: MempoolClient,
    ) -> anyhow::Result<Self> {
        let secp = Secp256k1::new();
        let xprv = Xpriv::new_master(Network::Bitcoin, &keypair.secret_bytes())?;
        let oracle = Oracle::new(storage.clone(), keypair.secret_key(), xprv);
        Ok(Self {
            oracle,
            pool,
            secp,
            pubkey: keypair.x_only_public_key().0,
            mempool,
        })
    }

    pub async fn attest_parlay_contract(&self, id: String) -> anyhow::Result<u64> {
        let contract = parlay::get_parlay_contract(self.pool.clone(), id).await?;
        let mut scores = Vec::new();
        for parameter in contract.parameters {
            let outcome = EventType::outcome(&parameter.data_type, &self.mempool).await?;
            println!("outcome {:?}", outcome);
            let normalized_value = parameter.normalize_parameter(outcome);
            println!("normalized value {:?}", normalized_value);
            let transformed_value = parameter.apply_transformation(normalized_value);
            println!("transformed value {:?}", transformed_value);
            // TODO: assert weights are correct.
            // let score = transformed_value * parameter.weight;
            scores.push(transformed_value);
        }
        let combined_score = parlay::combine_scores(&scores, &[], &contract.combination_method);
        let attestable_value =
            parlay::convert_to_attestable_value(combined_score, contract.max_normalized_value);
        Ok(attestable_value)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        mempool::MempoolClient,
        parlay::{CombinationMethod, ParlayContract},
        test_util::{setup_ernest_oracle, setup_mock_server_from_test_vectors, TestVectors},
    };
    use sqlx::PgPool;
    use std::{fs::read_to_string, str::FromStr};

    #[tokio::test]
    async fn test_attest_parlay_contract() {
        let test_vectors = read_to_string("../vectors.json").expect("Failed to read test vectors");
        let test_vectors: TestVectors =
            serde_json::from_str(&test_vectors).expect("Failed to parse test vectors");

        for test_vector in test_vectors.test_vectors {
            let mock_server = setup_mock_server_from_test_vectors(test_vector.clone()).await;
            let mempool = MempoolClient::new(format!("{}/api/v1", mock_server.uri()));
            let pg_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
            let pool = PgPool::connect(&pg_url)
                .await
                .expect("Failed to connect to database");
            let oracle = setup_ernest_oracle(mempool).await;
            let id = uuid::Uuid::new_v4().to_string();
            ParlayContract::new(
                pool.clone(),
                id.clone(),
                test_vector
                    .contract
                    .parameters
                    .into_iter()
                    .map(|p| p.into())
                    .collect(),
                CombinationMethod::from_str(&test_vector.contract.combination_method)
                    .expect("Failed to parse combination method"),
                test_vector.contract.max_normalized_value as u64,
            )
            .await
            .expect("could not create parlay contract");
            let attestable_value = oracle
                .attest_parlay_contract(id.clone())
                .await
                .expect("could not attest parlay contract");
            println!("attestable value {:?}", attestable_value);
            println!("expected {:?}", test_vector.expected.attestation_value);

            assert_eq!(attestable_value, test_vector.expected.attestation_value);
        }
    }
}
