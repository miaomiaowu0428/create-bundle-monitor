use create_bundle_monitor::BundleStore;
use solana_ix_collection::system_ix::cu_budget::SetComputUnitLimit;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Total bundles: {}", bundles.len());
    println!("🔍 Analyzing CU (Compute Unit) patterns...\n");

    // 统计：CU 值 -> (bundle 数量, mints 列表, follow数量分布)
    let mut cu_patterns: HashMap<Option<u32>, (usize, Vec<String>, HashMap<usize, usize>)> =
        HashMap::new();
    let mut no_cu_bundles = 0;
    let mut mixed_cu_bundles = 0;

    for bundle in &bundles {
        if bundle.follow_txs.is_empty() {
            no_cu_bundles += 1;
            continue;
        }

        // 提取所有 follow 交易的 CU 值
        let cu_values: Vec<Option<u32>> = bundle
            .follow_txs
            .iter()
            .map(|tx| {
                tx.flattened_ixs
                    .iter()
                    .find_map(|ix| SetComputUnitLimit::from_indexed_instruction(ix))
                    .map(|cu| cu.units)
            })
            .collect();

        // 检查是否所有交易都有 CU 且值相同
        if cu_values.iter().all(|cu| cu.is_some()) {
            let first_cu = cu_values[0];
            if cu_values.iter().all(|cu| *cu == first_cu) {
                // 所有交易 CU 一致
                let follow_count = bundle.follow_txs.len();
                let entry = cu_patterns
                    .entry(first_cu)
                    .or_insert((0, Vec::new(), HashMap::new()));
                entry.0 += 1;
                entry.1.push(bundle.mint.to_string());
                *entry.2.entry(follow_count).or_insert(0) += 1;
            } else {
                // 有 CU 但不一致
                mixed_cu_bundles += 1;
            }
        } else {
            // 部分交易没有 CU 指令
            mixed_cu_bundles += 1;
        }
    }

    // 排序并显示结果
    let mut sorted_patterns: Vec<_> = cu_patterns.iter().collect();
    sorted_patterns.sort_by_key(|(_, (count, _, _))| std::cmp::Reverse(*count));

    println!("═══════════════════════════════════════════════════════════");
    println!("📊 CU Pattern Distribution:");
    println!("═══════════════════════════════════════════════════════════\n");

    for (cu_opt, (count, mints, follow_dist)) in &sorted_patterns {
        if let Some(cu) = cu_opt {
            let percentage = (*count as f64 / bundles.len() as f64) * 100.0;
            println!(
                "🎯 CU Limit: {} ({} bundles, {:.2}%)",
                format_cu(*cu),
                count,
                percentage
            );

            // 显示 follow 交易数量分布
            let mut sorted_follow_dist: Vec<_> = follow_dist.iter().collect();
            sorted_follow_dist.sort_by_key(|(follow_count, _)| *follow_count);

            print!("   Follow txs distribution: ");
            let dist_parts: Vec<String> = sorted_follow_dist
                .iter()
                .map(|(follow_count, bundle_count)| {
                    format!("{}条: {}个", follow_count, bundle_count)
                })
                .collect();
            println!("{}", dist_parts.join(", "));

            // 显示前 5 个 mint
            let display_count = (*count).min(5);
            for mint in mints.iter().take(display_count) {
                println!("   • {}", mint);
            }
            if *count > 5 {
                println!("   ... and {} more", count - 5);
            }
            println!();
        }
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("📈 Summary:");
    println!("   Total bundles:              {}", bundles.len());
    println!("   Bundles with no follow txs: {}", no_cu_bundles);
    println!("   Bundles with mixed CU:      {}", mixed_cu_bundles);
    println!(
        "   Bundles with uniform CU:    {}",
        sorted_patterns
            .iter()
            .map(|(_, (c, _, _))| c)
            .sum::<usize>()
    );
    println!("   Unique CU patterns:         {}", sorted_patterns.len());

    // 高亮显示 140k
    if let Some((count, mints, follow_dist)) = cu_patterns.get(&Some(140_000)) {
        println!(
            "\n⭐ Special: {} bundles have ALL follow txs with 140k CU",
            count
        );

        // 显示 follow 分布
        let mut sorted_follow: Vec<_> = follow_dist.iter().collect();
        sorted_follow.sort_by_key(|(follow_count, _)| *follow_count);
        print!("   Follow distribution: ");
        let parts: Vec<String> = sorted_follow
            .iter()
            .map(|(fc, bc)| format!("{}条: {}个", fc, bc))
            .collect();
        println!("{}", parts.join(", "));

        println!("   First few mints:");
        for mint in mints.iter().take(10) {
            println!("   {}", mint);
        }
    }

    Ok(())
}

fn format_cu(cu: u32) -> String {
    if cu >= 1_000_000 {
        format!("{:.1}M", cu as f64 / 1_000_000.0)
    } else if cu >= 1_000 {
        format!("{}k", cu / 1_000)
    } else {
        cu.to_string()
    }
}
