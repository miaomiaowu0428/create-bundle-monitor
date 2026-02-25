use std::sync::Arc;

use create_bundle_monitor::monitor::PumpCreateBundleMonitor;
use dotenvy::dotenv;
use solana_sdk::pubkey;
use transaction_monitor::tx_dispatcher::TxDispatcher;
use utils::init_logger;

#[tokio::main]
async fn main() {
    // 1. 初始化 TLS
    rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider())
        .expect("Failed to install rustls crypto provider");
    println!("✅ TLS provider initialized");

    // 2. 加载环境变量
    dotenv().ok();
    println!("✅ Environment variables loaded");

    // 3. 初始化日志
    init_logger();
    log::info!("✅ Logger initialized");

    let dispatcher = TxDispatcher::new();

    // 4. 初始化monitor
    let monitor =
        PumpCreateBundleMonitor::new("./pump_bundles_db").expect("Failed to create monitor");
    log::info!("✅ Monitor initialized with database");

    // 5. 注册订阅者
    dispatcher.register(Arc::new(monitor));
    log::info!("✅ Monitor registered to dispatcher");

    // 6. 设置过滤器
    dispatcher.with_account_filters(vec![pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")]);

    log::info!("🚀 Starting dispatcher.run()...");

    // 7. 启动监听（会无限运行）
    dispatcher.run().await;
}
