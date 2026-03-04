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

    'bundle_loop: for bundle in &bundles {
        // 跳过没有 follow 交易的 bundle
        if bundle.follow_txs.is_empty() {
            continue;
        }
        let Some(cu_limit_ix) = bundle
            .create_tx
            .flattened_ixs
            .iter()
            .find_map(SetComputUnitLimit::from_indexed_instruction)
        else {
            continue;
        };
        if cu_limit_ix.units != 400_000 {
            continue;
        }
        let mut res = vec![];
        for tx in &bundle.follow_txs {
            let buy_count = tx
                .flattened_ixs
                .iter()
                .filter_map(PumpBuyIx::from_indexed_instruction)
                .collect::<Vec<_>>()
                .len();
            if buy_count < 2 {
                continue 'bundle_loop;
            }
            if let Some(cu_limit_ix) = tx
                .flattened_ixs
                .iter()
                .find_map(SetComputUnitLimit::from_indexed_instruction)
            {
                if cu_limit_ix.units != 150_000 * buy_count as u32 + 50_000 {
                    continue 'bundle_loop;
                }
                res.push((cu_limit_ix.units, buy_count));
            }
        }
        if res.is_empty() {
            continue;
        }
        println!("{};{:?}", bundle.mint, res);
    }

    Ok(())
}
