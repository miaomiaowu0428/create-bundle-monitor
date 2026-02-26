use create_bundle_monitor::{BundleStore, TxInfo};
use solana_ix_collection::system_ix::cu_budget::{SetComputUnitLimit, SetComputUnitPrice};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 从交易中提取 CU 信息
fn extract_cu_info(tx: &TxInfo) -> (Option<u32>, Option<u64>) {
    let cu_limit = tx
        .flattened_ixs
        .iter()
        .find_map(|ix| SetComputUnitLimit::from_indexed_instruction(ix))
        .map(|cu| cu.units);

    let cu_price = tx
        .flattened_ixs
        .iter()
        .find_map(|ix| SetComputUnitPrice::from_indexed_instruction(ix))
        .map(|cu| cu.micro_lamports);

    (cu_limit, cu_price)
}

/// 格式化 CU limit
fn format_cu_limit(cu: Option<u32>) -> String {
    match cu {
        Some(cu) if cu >= 1_000_000 => format!("{:.2}M", cu as f64 / 1_000_000.0),
        Some(cu) if cu >= 1_000 => format!("{}k", cu / 1_000),
        Some(cu) => cu.to_string(),
        None => "N/A".to_string(),
    }
}

/// 格式化 CU price (micro-lamports 转换为 lamports)
fn format_cu_price(price: Option<u64>) -> String {
    match price {
        Some(p) => format!("{} Lamports",  p as f64 / 1_000_000.0 ),
        None => "N/A".to_string(),
    }
}

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
            
            // 提取并显示 CU 信息
            let (cu_limit, cu_price) = extract_cu_info(&bundle.create_tx);
            println!("  CU Limit:  {}", format_cu_limit(cu_limit));
            println!("  CU Price:  {}", format_cu_price(cu_price));

            println!("\n📝 FOLLOW TRANSACTIONS ({}):", bundle.follow_txs.len());
            for (i, tx) in bundle.follow_txs.iter().enumerate() {
                println!("\n  [{}] Transaction:", i + 1);
                println!("    Signature: {}", tx.signature);
                println!("    Slot:      {}", tx.slot);
                println!("    Index:     {}", tx.index);
                println!("    Accounts:  {} accounts", tx.account_keys.len());
                println!("    Instructions: {} instructions", tx.flattened_ixs.len());
                
                // 提取并显示 CU 信息
                let (cu_limit, cu_price) = extract_cu_info(tx);
                println!("    CU Limit:  {}", format_cu_limit(cu_limit));
                println!("    CU Price:  {}", format_cu_price(cu_price));
            }

            println!("\n{}", "=".repeat(80));
        }
        None => {
            println!("❌ No bundle found for mint: {}", mint);
        }
    }

    Ok(())
}
