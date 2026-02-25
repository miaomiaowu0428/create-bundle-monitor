use create_bundle_monitor::BundleStore;
use serde_json;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    let output_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "./bundles_export.json".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Found {} bundles", bundles.len());
    println!("💾 Exporting to: {}", output_path);

    let json = serde_json::to_string_pretty(&bundles)?;
    let mut file = File::create(&output_path)?;
    file.write_all(json.as_bytes())?;

    println!("✅ Export complete!");

    Ok(())
}
