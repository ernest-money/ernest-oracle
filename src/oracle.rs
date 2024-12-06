use crate::storage::PostgresStorage;
use bitcoin::{
    bip32::Xpriv,
    key::{Keypair, Secp256k1},
    secp256k1::All,
    Network, XOnlyPublicKey,
};
use kormir::Oracle;

pub struct ErnestOracle {
    pub oracle: Oracle<PostgresStorage>,
    pubkey: XOnlyPublicKey,
    secp: Secp256k1<All>,
}

impl ErnestOracle {
    pub fn new(storage: PostgresStorage, keypair: Keypair) -> anyhow::Result<Self> {
        let secp = Secp256k1::new();
        let xprv = Xpriv::new_master(Network::Bitcoin, &keypair.secret_bytes())?;
        let oracle = Oracle::new(storage, keypair.secret_key(), xprv);
        Ok(Self {
            oracle,
            secp,
            pubkey: keypair.x_only_public_key().0,
        })
    }
}
