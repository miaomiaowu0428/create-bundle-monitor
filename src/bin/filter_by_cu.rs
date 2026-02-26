use create_bundle_monitor::BundleStore;
use solana_ix_collection::system_ix::cu_budget::SetComputUnitLimit;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cu_target: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(140_000);

    let db_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);
    println!("🎯 Target CU limit: {}\n", cu_target);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Total bundles: {}", bundles.len());
    println!("🔍 Filtering bundles where ALL follow txs have CU limit = {}...\n", cu_target);

    let mut matched_mints = Vec::new();

    for bundle in &bundles {
        // 跳过没有 follow 交易的 bundle
        if bundle.follow_txs.is_empty() {
            continue;
        }

        // 检查所有 follow 交易是否都有目标 CU limit
        let all_match = bundle.follow_txs.iter().all(|tx| {
            tx.flattened_ixs.iter().any(|ix| {
                if let Some(cu_limit) = SetComputUnitLimit::from_indexed_instruction(ix) {
                    cu_limit.units == cu_target
                } else {
                    false
                }
            })
        });

        if all_match {
            matched_mints.push(bundle.mint);
            println!("✅ Mint: {}", bundle.mint);
            println!("   Create tx: {}", bundle.create_tx.signature);
            println!("   Follow txs: {}", bundle.follow_txs.len());
            
            // 显示每个 follow 交易的详细信息
            for (i, tx) in bundle.follow_txs.iter().enumerate() {
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
    println!("   Target CU limit:  {}", cu_target);
    println!("   Total bundles:    {}", bundles.len());
    println!("   Matched bundles:  {}", matched_mints.len());
    println!("   Match rate:       {:.2}%", 
        if bundles.is_empty() { 0.0 } else { (matched_mints.len() as f64 / bundles.len() as f64) * 100.0 }
    );
    
    if !matched_mints.is_empty() {
        println!("\n📋 Matched mints (plain list):");
        for mint in &matched_mints {
            println!("{}", mint);
        }
    }

    Ok(())
}
