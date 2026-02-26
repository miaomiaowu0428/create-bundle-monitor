use create_bundle_monitor::{BundleStore, TxBundle};
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json_path = std::env::args()
        .nth(1)
        .expect("Usage: migrate <bundles.json> [new_db_path]");

    let db_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "./pump_bundles_db_lmdb".to_string());

    println!("📁 Reading from: {}", json_path);
    println!("💾 Migrating to: {}", db_path);

    // 读取 JSON
    let file = File::open(&json_path)?;
    let reader = BufReader::new(file);
    let bundles: Vec<TxBundle> = serde_json::from_reader(reader)?;

    println!("📦 Found {} bundles to migrate", bundles.len());

    // 创建新数据库
    let store = BundleStore::open(&db_path)?;

    // 迁移数据
    for (i, bundle) in bundles.iter().enumerate() {
        store.store(bundle)?;
        if (i + 1) % 100 == 0 {
            println!("  Migrated {}/{} bundles...", i + 1, bundles.len());
        }
    }

    println!("✅ Migration complete! {} bundles migrated", bundles.len());
    println!("\n🎯 Next steps:");
    println!("  1. Verify migration: cargo run --bin stats {}", db_path);
    println!("  2. Update your config to use the new database path");
    println!("  3. Backup old database: mv pump_bundles_db pump_bundles_db.sled.backup");

    Ok(())
}
