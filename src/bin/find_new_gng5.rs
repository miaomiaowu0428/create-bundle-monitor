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
        let mut res = vec![];
        for tx in &bundle.follow_txs {
            let Some(buy) = tx
                .flattened_ixs
                .iter()
                .find_map(PumpBuyIx::from_indexed_instruction)
            else {
                return;
            };
            if buy.max_sol_cost == 15544500000 || buy.max_sol_cost == 8445000000 {
                res.push(buy.max_sol_cost);
            }
        }
        if res.len() != 2 {
            return;
        }
        println!("{};\t\t{:?}", bundle.mint, res);
    });

    Ok(())
}
