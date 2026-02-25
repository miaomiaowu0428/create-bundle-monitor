use create_bundle_monitor::BundleStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("\n📊 DATABASE STATISTICS");
    println!("{}", "=".repeat(60));
    println!("Total bundles: {}", bundles.len());

    let mut follow_tx_counts = vec![0; 5]; // 0-4笔后续交易
    let mut total_follow_txs = 0;
    let mut min_slot = u64::MAX;
    let mut max_slot = 0u64;

    for bundle in &bundles {
        let count = bundle.follow_txs.len();
        if count <= 4 {
            follow_tx_counts[count] += 1;
        }
        total_follow_txs += count;

        min_slot = min_slot.min(bundle.create_tx.slot);
        max_slot = max_slot.max(bundle.create_tx.slot);
    }

    println!("\n📈 Follow Transaction Distribution:");
    println!("{}", "-".repeat(60));
    for (count, num_bundles) in follow_tx_counts.iter().enumerate() {
        if *num_bundles > 0 {
            let percentage = (*num_bundles as f64 / bundles.len() as f64) * 100.0;
            println!(
                "  {} follow txs: {:>5} bundles ({:>5.1}%)",
                count, num_bundles, percentage
            );
        }
    }

    if !bundles.is_empty() {
        let avg_follow_txs = total_follow_txs as f64 / bundles.len() as f64;
        println!("\n📊 Averages:");
        println!("{}", "-".repeat(60));
        println!("  Average follow txs per bundle: {:.2}", avg_follow_txs);

        println!("\n🕐 Slot Range:");
        println!("{}", "-".repeat(60));
        println!("  Min slot: {}", min_slot);
        println!("  Max slot: {}", max_slot);
        println!("  Range:    {}", max_slot - min_slot);
    }

    println!("\n{}", "=".repeat(60));

    Ok(())
}
