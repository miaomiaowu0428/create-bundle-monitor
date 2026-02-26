use create_bundle_monitor::BundleStore;
use solana_ix_collection::system_ix::cu_budget::SetComputUnitLimit;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Total bundles: {}", bundles.len());
    println!("🔍 Filtering bundles where ALL follow txs have CU limit = 140k...\n");

    let mut matched_count = 0;

    for bundle in &bundles {
        // 跳过没有 follow 交易的 bundle
        if bundle.follow_txs.is_empty() {
            continue;
        }

        // 检查所有 follow 交易是否都有 140k CU limit
        let all_140k = bundle.follow_txs.iter().all(|tx| {
            // 遍历交易中的所有指令，查找 SetComputUnitLimit
            tx.flattened_ixs.iter().any(|ix| {
                if let Some(cu_limit) = SetComputUnitLimit::from_indexed_instruction(ix) {
                    cu_limit.units == 140_000
                } else {
                    false
                }
            })
        });

        if all_140k {
            matched_count += 1;
            println!("✅ Mint: {}", bundle.mint);
            println!("   Create tx: {}", bundle.create_tx.signature);
            println!("   Follow txs: {}", bundle.follow_txs.len());
            
            // 显示每个 follow 交易的详细信息
            for (i, tx) in bundle.follow_txs.iter().enumerate() {
                // 找到 CU limit 指令并显示
                let cu_info = tx.flattened_ixs.iter()
                    .find_map(|ix| SetComputUnitLimit::from_indexed_instruction(ix))
                    .map(|cu| cu.units)
                    .unwrap_or(0);
                
                println!("     [{}] {} - CU: {}", i + 1, tx.signature, cu_info);
            }
            println!();
        }
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("📊 Summary:");
    println!("   Total bundles:   {}", bundles.len());
    println!("   Matched (140k):  {}", matched_count);
    println!("   Match rate:      {:.2}%", 
        if bundles.is_empty() { 0.0 } else { (matched_count as f64 / bundles.len() as f64) * 100.0 }
    );

    Ok(())
}
