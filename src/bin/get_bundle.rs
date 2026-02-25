use create_bundle_monitor::BundleStore;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mint_str = std::env::args()
        .nth(1)
        .expect("Usage: get_bundle <mint_address> [db_path]");

    let db_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let mint = Pubkey::from_str(&mint_str)?;
    let store = BundleStore::open(&db_path)?;

    match store.get(&mint)? {
        Some(bundle) => {
            println!("\n✅ Found bundle for mint: {}", bundle.mint);
            println!("{}", "=".repeat(80));

            println!("\n📦 CREATE TRANSACTION:");
            println!("  Signature: {}", bundle.create_tx.signature);
            println!("  Slot:      {}", bundle.create_tx.slot);
            println!("  Index:     {}", bundle.create_tx.index);
            println!(
                "  Accounts:  {} accounts",
                bundle.create_tx.account_keys.len()
            );
            println!(
                "  Instructions: {} instructions",
                bundle.create_tx.flattened_ixs.len()
            );

            println!("\n📝 FOLLOW TRANSACTIONS ({}):", bundle.follow_txs.len());
            for (i, tx) in bundle.follow_txs.iter().enumerate() {
                println!("\n  [{}] Transaction:", i + 1);
                println!("    Signature: {}", tx.signature);
                println!("    Slot:      {}", tx.slot);
                println!("    Index:     {}", tx.index);
                println!("    Accounts:  {} accounts", tx.account_keys.len());
                println!("    Instructions: {} instructions", tx.flattened_ixs.len());
            }

            println!("\n{}", "=".repeat(80));
        }
        None => {
            println!("❌ No bundle found for mint: {}", mint);
        }
    }

    Ok(())
}
