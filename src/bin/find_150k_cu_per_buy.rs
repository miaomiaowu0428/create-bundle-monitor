use create_bundle_monitor::BundleStore;
use solana_ix_collection::{pump::PumpBuyIx, system_ix::cu_budget::SetComputUnitLimit};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Total bundles: {}", bundles.len());

    for bundle in &bundles {
        // 跳过没有 follow 交易的 bundle
        if bundle.follow_txs.is_empty() {
            continue;
        }
        if bundle
            .create_tx
            .flattened_ixs
            .iter()
            .find_map(SetComputUnitLimit::from_indexed_instruction)
            .map(|ix| ix.units == 400_000)
            .unwrap_or(false)
        {
            return Ok(());
        }
        for tx in bundle.follow_txs {
            let buy_count = tx
                .flattened_ixs
                .iter()
                .filter_map(PumpBuyIx::from_indexed_instruction)
                .collect::<Vec<_>>()
                .len();
            if let Some(cu_limit_ix) = tx
                .flattened_ixs
                .iter()
                .find_map(SetComputUnitLimit::from_indexed_instruction)
            {
                if cu_limit_ix.units == 150_000 * buy_count as u64 + 50000 {
                    println!("{}", bundle.mint);
                }
            }
        }
    }

    Ok(())
}
