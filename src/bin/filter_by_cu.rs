use create_bundle_monitor::BundleStore;
use solana_ix_collection::system_ix::cu_budget::SetComputUnitLimit;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let mut cu_target: u32 = 140_000;
    let mut db_path = "./pump_bundles_db".to_string();
    let mut follow_count: Option<usize> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--follow-count" => {
                if i + 1 < args.len() {
                    follow_count = args[i + 1].parse().ok();
                    i += 2;
                } else {
                    eprintln!("⚠️  --follow-count requires a value");
                    i += 1;
                }
            }
            arg => {
                // 位置参数：先是 cu_target，再是 db_path
                if arg.parse::<u32>().is_ok() && cu_target == 140_000 {
                    cu_target = arg.parse().unwrap();
                } else if !arg.starts_with("--") {
                    db_path = arg.to_string();
                }
                i += 1;
            }
        }
    }

    println!("📁 Opening database: {}", db_path);
    println!("🎯 Target CU limit: {}", cu_target);
    if let Some(count) = follow_count {
        println!("📊 Follow count filter: exactly {} txs", count);
    }
    println!();

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Total bundles: {}", bundles.len());
    print!(
        "🔍 Filtering bundles where ALL follow txs have CU limit = {}",
        cu_target
    );
    if let Some(count) = follow_count {
        print!(" AND follow count = {}", count);
    }
    println!("...\n");

    let mut matched_mints = Vec::new();

    for bundle in &bundles {
        // 跳过没有 follow 交易的 bundle
        if bundle.follow_txs.is_empty() {
            continue;
        }

        // 检查 follow 数量（如果指定了过滤条件）
        if let Some(expected_count) = follow_count {
            if bundle.follow_txs.len() != expected_count {
                continue;
            }
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
                let cu_info = tx
                    .flattened_ixs
                    .iter()
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
    if let Some(count) = follow_count {
        println!("   Follow count:     {}", count);
    }
    println!("   Total bundles:    {}", bundles.len());
    println!("   Matched bundles:  {}", matched_mints.len());
    println!(
        "   Match rate:       {:.2}%",
        if bundles.is_empty() {
            0.0
        } else {
            (matched_mints.len() as f64 / bundles.len() as f64) * 100.0
        }
    );

    if !matched_mints.is_empty() {
        println!("\n📋 Matched mints (plain list):");
        for mint in &matched_mints {
            println!("{}", mint);
        }
    }

    Ok(())
}
