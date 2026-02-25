use create_bundle_monitor::BundleStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let mints = store.list_mints()?;

    println!("\n📊 Total bundles: {}", mints.len());
    println!("\n🪙 Mint addresses:");
    println!("{}", "=".repeat(60));

    for (i, mint) in mints.iter().enumerate() {
        println!("{}. {}", i + 1, mint);
    }

    Ok(())
}
