use std::sync::Arc;

use grpc_client::TransactionFormat;
use log::{info, warn};
use solana_ix_collection::pump::PumpMigrateIx;
use transaction_monitor::tx_subscriber::TxSubscriber;
use utils::flatten_instructions;

use crate::{MigrateRecord, MigrateStore};

pub struct PumpMigrateMonitor {
    store: Arc<MigrateStore>,
}

impl PumpMigrateMonitor {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let store = Arc::new(MigrateStore::open(db_path)?);
        Ok(Self { store })
    }
}

#[async_trait::async_trait]
impl TxSubscriber for PumpMigrateMonitor {
    fn name(&self) -> &'static str {
        "PumpMigrateMonitor"
    }

    async fn interested(&self, tx: &TransactionFormat) -> Option<bool> {
        Some(tx.account_keys.contains(&solana_sdk::pubkey!(
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
        )))
    }

    async fn on_tx(self: Arc<Self>, tx: Arc<TransactionFormat>) {
        let ixs = flatten_instructions(&tx);

        let Some(migrate_ix) = ixs
            .iter()
            .find_map(|ix| PumpMigrateIx::from_indexed_instruction(ix))
        else {
            return;
        };

        let mint = migrate_ix.mint;
        let record = MigrateRecord {
            mint,
            signature: tx.signature,
            slot: tx.slot,
        };

        info!(
            "🔀 Migrate detected: mint={}, sig={}, slot={}",
            mint, tx.signature, tx.slot
        );

        if let Err(e) = self.store.store(&record) {
            warn!("❌ Failed to store migrate record for {}: {}", mint, e);
        }
    }
}
