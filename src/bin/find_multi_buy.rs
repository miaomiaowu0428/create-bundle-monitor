use clap::Parser;

use create_bundle_monitor::BundleStore;
use dotenvy::dotenv;
use log::info;
use solana_ix_collection::pump::PumpBuyIx;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use utils::init_logger;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the pump bundles database
    #[arg(default_value = "./pump_bundles_db")]
    db_path: String,

    /// Minimum number of pump-buy instructions in follow transactions
    #[arg(long, short = 'n', default_value_t = 0)]
    min_follow_buy: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    init_logger();
    let args = Cli::parse();
    let db_path = args.db_path;
    let min_follow_buy = args.min_follow_buy;

    info!(
        "🔢 Requiring more than {} pump-buy instructions in follow transactions",
        min_follow_buy
    );

    info!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    info!("📦 Total bundles: {}", bundles.len());
    info!(
        "🔍 Filtering bundles with > {} pump-buy instructions in follow txs\n",
        min_follow_buy
    );

    let mut matched_count = 0;
    let mut matched_pairs: Vec<(Pubkey, Signature)> = Vec::new();

    for bundle in &bundles {
        // 跳过没有 follow 交易的 bundle
        if bundle.follow_txs.is_empty() {
            continue;
        }

        // 统计 follow 交易中匹配到的 PumpBuyIx 数量
        let mut follow_buy_count = 0usize;
        for tx in &bundle.follow_txs {
            follow_buy_count += tx
                .flattened_ixs
                .iter()
                .filter_map(PumpBuyIx::from_indexed_instruction)
                .count();
        }
        if follow_buy_count <= min_follow_buy {
            // 未达到阈值，跳过
            continue;
        }

        let sig = bundle.create_tx.signature;
        // record the mint/signature pair for output later
        matched_pairs.push((bundle.mint, sig));

        matched_count += 1;
        info!("✅ Mint: {}", bundle.mint);
        info!("   Create tx: {}", bundle.create_tx.signature);
        info!(
            "   Follow txs: {} (pump-buy instructions: {})",
            bundle.follow_txs.len(),
            follow_buy_count
        );

        // ========== 新增：解析 create_tx 中的 PumpBuyIx 指令 ==========
        // 提取 create_tx 中的所有 buy 指令
        let create_buy_ixs: Vec<PumpBuyIx> = bundle
            .create_tx
            .flattened_ixs
            .iter()
            .filter_map(PumpBuyIx::from_indexed_instruction)
            .collect();

        // 统计 create_tx 中的 token_amount
        let mut create_token_amount = 0u64;

        // 输出 create_tx 中的 buy 指令详情
        if !create_buy_ixs.is_empty() {
            for buy_ix in create_buy_ixs.iter() {
                create_token_amount += buy_ix.token_amount;
            }
        }

        // ========== 调整总计初始值：包含 create_tx 的统计 ==========
        let mut total_token_amount = create_token_amount; // 初始化为 create_tx 的 token 量
        let mut follow_amounts: Vec<f64> = Vec::new(); // 记录每个 follow tx 的 token 量

        // 显示每个 follow 交易的详细信息
        for tx in bundle.follow_txs.iter() {
            // 找到所有 PumpBuyIx 指令
            let buy_ixs: Vec<PumpBuyIx> = tx
                .flattened_ixs
                .iter()
                .filter_map(PumpBuyIx::from_indexed_instruction)
                .collect();

            let mut tx_token_amount = 0u64;
            if !buy_ixs.is_empty() {
                for buy_ix in buy_ixs.iter() {
                    tx_token_amount += buy_ix.token_amount;
                }
            }

            if tx_token_amount > 0 {
                follow_amounts.push(tx_token_amount as f64 / 1_000_000_000_000.0);
                total_token_amount += tx_token_amount;
            }
        }

        // ========== 更新汇总输出：明确区分 create 和 follow 的贡献 ==========
        let create_m = create_token_amount as f64 / 1_000_000_000_000.0;
        let total_m = total_token_amount as f64 / 1_000_000_000_000.0;

        // 构建格式化字符串：create + follow1 + follow2 + ...
        let mut amount_parts = vec![format!("{:.2}M", create_m)];
        for amount in follow_amounts {
            amount_parts.push(format!("{:.2}M", amount));
        }
        let amount_breakdown = amount_parts.join(" + ");

        info!("   📊 Summary:");
        info!(
            "      Total token_amount: {:.2}M ({})",
            total_m, amount_breakdown
        );
        info!("");
    }

    info!("═══════════════════════════════════════════════════════════");
    info!("📊 Final Summary:");
    info!("   Total bundles:         {}", bundles.len());
    info!(
        "   Matched (follow buy > {}):        {}",
        min_follow_buy, matched_count
    );
    info!(
        "   Match rate:            {:.2}%",
        if bundles.is_empty() {
            0.0
        } else {
            (matched_count as f64 / bundles.len() as f64) * 100.0
        }
    );

    if !matched_pairs.is_empty() {
        info!("🔗 Matched token:sig pairs:");
        for (mint, sig) in &matched_pairs {
            info!("   {} : {}", mint, sig);
        }
    }

    Ok(())
}
