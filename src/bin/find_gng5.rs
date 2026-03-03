use create_bundle_monitor::BundleStore;
use solana_ix_collection::pump::PumpBuyIx;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./pump_bundles_db".to_string());

    println!("📁 Opening database: {}", db_path);

    let store = BundleStore::open(&db_path)?;
    let bundles = store.list_all()?;

    println!("📦 Total bundles: {}", bundles.len());
    println!("🔍 Filtering GNG5 bundles (max_sol_cost = u64::MAX)...\n");

    let mut matched_count = 0;

    for bundle in &bundles {
        // 跳过没有 follow 交易的 bundle
        if bundle.follow_txs.is_empty() {
            continue;
        }

        // 检查所有 follow 交易是否都有 140k CU limit
        let is_gng5 = bundle.follow_txs[0]
            .flattened_ixs
            .iter()
            .find_map(PumpBuyIx::from_indexed_instruction)
            .filter(|ix| ix.max_sol_cost == u64::MAX)
            .is_some();

        if is_gng5 {
            matched_count += 1;
            println!("✅ Mint: {}", bundle.mint);
            println!("   Create tx: {}", bundle.create_tx.signature);
            println!("   Follow txs: {}", bundle.follow_txs.len());

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

            println!("   📊 Summary:");
            println!(
                "      Total token_amount: {:.2}M ({})",
                total_m, amount_breakdown
            );
            println!();
        }
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("📊 Final Summary:");
    println!("   Total bundles:         {}", bundles.len());
    println!("   Matched (GNG5):        {}", matched_count);
    println!(
        "   Match rate:            {:.2}%",
        if bundles.is_empty() {
            0.0
        } else {
            (matched_count as f64 / bundles.len() as f64) * 100.0
        }
    );

    Ok(())
}
