use create_bundle_monitor::BundleStore;
use solana_ix_collection::system_ix::cu_budget::SetComputUnitPrice;
use solana_ix_collection::{
    pump::{PumpBuyExactInIx, PumpBuyIx},
    system_ix::cu_budget::SetComputUnitLimit,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Total bundles: {}", bundles.len());

    bundles.iter().for_each(|bundle| {
        if bundle.follow_txs.is_empty() {
            return;
        }
        let Some(cu_limit_ix) = bundle
            .create_tx
            .flattened_ixs
            .iter()
            .find_map(SetComputUnitLimit::from_indexed_instruction)
        else {
            return;
        };
        let None = bundle
            .create_tx
            .flattened_ixs
            .iter()
            .find_map(SetComputUnitPrice::from_indexed_instruction)
        else {
            return;
        };
        if cu_limit_ix.units != 400_000 {
            return;
        }
        let mut res = vec![];
        for tx in &bundle.follow_txs {
            let Some(buy_exact_sol_in) = tx
                .flattened_ixs
                .iter()
                .find_map(PumpBuyExactInIx::from_indexed_instruction)
            else {
                return;
            };
            if buy_exact_sol_in.min_token_out != 1 {
                return;
            }
            let None = tx
                .flattened_ixs
                .iter()
                .find_map(SetComputUnitPrice::from_indexed_instruction)
            else {
                return;
            };
            if let Some(cu_limit_ix) = tx
                .flattened_ixs
                .iter()
                .find_map(SetComputUnitLimit::from_indexed_instruction)
            {
                if cu_limit_ix.units != 400_000 {
                    return;
                }
                res.push((cu_limit_ix.units, buy_exact_sol_in.min_token_out));
            }
        }
        if res.len() < 2 {
            return;
        }
        println!("{};\t\t{:?}", bundle.mint, res);
    });

    Ok(())
}
