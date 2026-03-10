use create_bundle_monitor::MigrateStore;
use serde_json;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 接收一个 JSON 文件，内容为 mint 地址数组，输出其中已迁移的 mint。
///
/// 用法：check_migrated [db_path] [mints_json]
///   db_path    - 数据库目录，默认 ./migrate_db
///   mints_json - JSON 文件路径，默认 ./mints.json
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./migrate_db".to_string());
    let json_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "./mints.json".to_string());

    println!("📁 Opening database: {}", db_path);
    let store = MigrateStore::open(&db_path)?;

    println!("📂 Reading mints from: {}", json_path);
    let raw = std::fs::read_to_string(&json_path)?;
    let mint_strs: Vec<String> = serde_json::from_str(&raw)?;

    let total = mint_strs.len();
    let mut migrated = Vec::new();
    let mut parse_errors = 0usize;

    for s in &mint_strs {
        let Ok(mint) = Pubkey::from_str(s) else {
            eprintln!("⚠️  Invalid pubkey: {}", s);
            parse_errors += 1;
            continue;
        };
        if store.contains(&mint)? {
            migrated.push(mint.to_string());
        }
    }

    println!("\n📊 Results");
    println!("{}", "=".repeat(60));
    println!("  Total input mints : {}", total);
    if parse_errors > 0 {
        println!("  Parse errors      : {}", parse_errors);
    }
    println!("  Migrated          : {}", migrated.len());
    println!("{}", "=".repeat(60));

    for (i, m) in migrated.iter().enumerate() {
        println!("{}. {}", i + 1, m);
    }

    Ok(())
}
