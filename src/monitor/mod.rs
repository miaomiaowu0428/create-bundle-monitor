use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};

use grpc_client::TransactionFormat;
use log::{debug, info, warn};
use solana_ix_collection::pump::PumpCreateIxEnum;
use solana_sdk::pubkey::Pubkey;
use transaction_monitor::tx_subscriber::TxSubscriber;
use utils::flatten_instructions;

// 从lib导入数据结构
use crate::{BundleStore, TxBundle, TxInfo};

/// 用于优先队列的交易包装器，实现按(slot, index)排序的小根堆
#[derive(Clone)]
struct OrderedTx {
    slot: u64,
    index: u64,
    tx: Arc<TransactionFormat>,
}

impl Eq for OrderedTx {}

impl PartialEq for OrderedTx {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && self.index == other.index
    }
}

impl Ord for OrderedTx {
    fn cmp(&self, other: &Self) -> Ordering {
        // 反向比较实现小根堆：先比较slot，再比较index
        match other.slot.cmp(&self.slot) {
            Ordering::Equal => other.index.cmp(&self.index),
            ord => ord,
        }
    }
}

impl PartialOrd for OrderedTx {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 待完成的bundle（还在收集后续交易）
#[derive(Debug, Clone)]
struct PendingBundle {
    mint: Pubkey,
    create_slot: u64,
    create_index: u64,
    create_tx: TxInfo,
    follow_txs: Vec<TxInfo>,
    target_count: usize,             // 需要收集的后续交易数量（4笔）
    last_update: std::time::Instant, // 最后更新时间，用于超时清理
}

pub struct PumpCreateBundleMonitor {
    store: Arc<BundleStore>,
    // 交易优先队列缓冲区（小根堆）
    tx_buffer: Arc<Mutex<BinaryHeap<OrderedTx>>>,
    // 使用mint作为key追踪待完成的bundle
    pending_bundles: Arc<Mutex<HashMap<Pubkey, PendingBundle>>>,
    // 当前已处理到的最大slot，用于判断何时可以安全处理缓冲区的交易
    max_processed_slot: Arc<Mutex<u64>>,
    // 已入队的交易总数（用于实现缓冲机制）
    enqueued_count: Arc<AtomicUsize>,
}

impl PumpCreateBundleMonitor {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let store = Arc::new(BundleStore::open(db_path)?);
        let monitor = Self {
            store: store.clone(),
            tx_buffer: Arc::new(Mutex::new(BinaryHeap::new())),
            pending_bundles: Arc::new(Mutex::new(HashMap::new())),
            max_processed_slot: Arc::new(Mutex::new(0)),
            enqueued_count: Arc::new(AtomicUsize::new(0)),
        };

        // 启动后台处理任务
        monitor.start_processing_task();

        Ok(monitor)
    }

    /// 启动后台任务处理优先队列中的交易
    fn start_processing_task(&self) {
        let tx_buffer = self.tx_buffer.clone();
        let pending_bundles = self.pending_bundles.clone();
        let max_processed_slot = self.max_processed_slot.clone();
        let enqueued_count = self.enqueued_count.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

            loop {
                interval.tick().await;

                // 检查是否已经积累了足够的交易（前100条只入队不处理）
                let current_enqueued = enqueued_count.load(AtomicOrdering::Relaxed);
                if current_enqueued < 100 {
                    debug!(
                        "📥 Buffering: {}/100 transactions enqueued",
                        current_enqueued
                    );
                    continue;
                }

                // 获取当前最大slot
                let current_max_slot = *max_processed_slot.lock().unwrap();

                // 从缓冲区取出可以安全处理的交易
                // 策略：只处理slot < current_max_slot - 2 的交易，留2个slot的缓冲
                loop {
                    let tx_to_process = {
                        let mut buffer = tx_buffer.lock().unwrap();
                        if let Some(ordered_tx) = buffer.peek() {
                            // 如果最小的交易slot太新，暂停处理
                            if ordered_tx.slot + 2 > current_max_slot {
                                break;
                            }
                            buffer.pop()
                        } else {
                            break;
                        }
                    };

                    if let Some(ordered_tx) = tx_to_process {
                        Self::process_tx_internal(ordered_tx.tx, &pending_bundles, &store);
                    }
                }

                // 清理超时的pending bundles（超过10秒未更新）
                Self::cleanup_timeout_bundles(&pending_bundles, &store);
            }
        });
    }

    /// 从交易中提取TxInfo
    fn extract_tx_info(tx: &TransactionFormat) -> TxInfo {
        TxInfo {
            signature: tx.signature,
            slot: tx.slot,
            index: tx.index,
            account_keys: tx.account_keys.clone(),
            flattened_ixs: flatten_instructions(tx),
        }
    }

    /// 内部处理交易的逻辑
    fn process_tx_internal(
        tx: Arc<TransactionFormat>,
        pending_bundles: &Arc<Mutex<HashMap<Pubkey, PendingBundle>>>,
        store: &Arc<BundleStore>,
    ) {
        let ixs = flatten_instructions(&tx);

        // 1. 检查是否是create指令
        if let Some(create_ix) = ixs
            .iter()
            .find_map(|ix| PumpCreateIxEnum::try_from(ix).ok())
        {
            let mint = create_ix.mint();

            info!(
                "🎯 Found PumpCreate: mint={}, slot={}, index={}, sig={}",
                mint, tx.slot, tx.index, tx.signature
            );

            let tx_info = Self::extract_tx_info(&tx);

            // 创建待完成的bundle
            let pending = PendingBundle {
                mint,
                create_slot: tx.slot,
                create_index: tx.index,
                create_tx: tx_info,
                follow_txs: Vec::new(),
                target_count: 4,
                last_update: std::time::Instant::now(),
            };

            let mut bundles = pending_bundles.lock().unwrap();
            bundles.insert(mint, pending);
            return;
        }

        // 2. 检查是否是待完成bundle的后续交易
        let mut bundles = pending_bundles.lock().unwrap();
        let mut completed_mints = Vec::new();

        for (mint, pending) in bundles.iter_mut() {
            // 检查条件：同slot，连续index，包含mint
            let is_same_slot = tx.slot == pending.create_slot;
            let expected_index = pending.create_index + 1 + pending.follow_txs.len() as u64;
            let is_consecutive_index = tx.index == expected_index;
            let contains_mint = tx.account_keys.contains(mint);

            // 如果同slot但index不连续，说明bundle完成了（index中断）
            if is_same_slot && !is_consecutive_index {
                debug!(
                    "🔄 Index gap detected for mint={}, expected_index={}, actual={}, completing bundle with {} follow txs",
                    mint,
                    expected_index,
                    tx.index,
                    pending.follow_txs.len()
                );
                completed_mints.push(*mint);
                continue; // 检查下一个mint
            }

            if is_same_slot && is_consecutive_index && contains_mint {
                let tx_info = Self::extract_tx_info(&tx);
                pending.follow_txs.push(tx_info);
                pending.last_update = std::time::Instant::now();

                debug!(
                    "📦 Added follow tx for mint={}, current count={}/{}, sig={}",
                    mint,
                    pending.follow_txs.len(),
                    pending.target_count,
                    tx.signature
                );

                // 检查是否收集完成（达到4笔）
                if pending.follow_txs.len() >= pending.target_count {
                    completed_mints.push(*mint);
                }
                break; // 一笔交易只能属于一个bundle
            }
        }

        // 3. 存储完成的bundle
        for mint in completed_mints {
            if let Some(pending) = bundles.remove(&mint) {
                let bundle = TxBundle {
                    mint: pending.mint,
                    create_tx: pending.create_tx,
                    follow_txs: pending.follow_txs,
                };

                if let Err(e) = store.store(&bundle) {
                    warn!("❌ Failed to store bundle for mint {}: {}", mint, e);
                } else {
                    info!(
                        "✅ Stored bundle for mint: {}, create_tx: {}, follow_txs: {}",
                        bundle.mint,
                        bundle.create_tx.signature,
                        bundle.follow_txs.len()
                    );
                }
            }
        }
    }

    /// 清理超时的pending bundles
    fn cleanup_timeout_bundles(
        pending_bundles: &Arc<Mutex<HashMap<Pubkey, PendingBundle>>>,
        store: &Arc<BundleStore>,
    ) {
        let timeout_duration = std::time::Duration::from_secs(10);
        let mut bundles = pending_bundles.lock().unwrap();
        let mut to_store = Vec::new();

        bundles.retain(|mint, pending| {
            if pending.last_update.elapsed() > timeout_duration {
                // 超时了，无论有没有后续交易都保存
                debug!(
                    "⏰ Timeout for mint={}, collected {}/{} follow txs, storing anyway",
                    mint,
                    pending.follow_txs.len(),
                    pending.target_count
                );

                // 无论有没有后续交易都保存
                to_store.push(pending.clone());

                false // 移除此pending
            } else {
                true // 保留
            }
        });

        // 释放锁后再存储
        drop(bundles);

        for pending in to_store {
            let bundle = TxBundle {
                mint: pending.mint,
                create_tx: pending.create_tx,
                follow_txs: pending.follow_txs,
            };

            if let Err(e) = store.store(&bundle) {
                warn!("❌ Failed to store timeout bundle: {}", e);
            } else {
                info!(
                    "✅ Stored bundle for mint: {}, create_tx: {}, follow_txs: {}",
                    bundle.mint,
                    bundle.create_tx.signature,
                    bundle.follow_txs.len()
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl TxSubscriber for PumpCreateBundleMonitor {
    fn name(&self) -> &'static str {
        "PumpCreateBundleMonitor"
    }

    async fn interested(&self, tx: &TransactionFormat) -> Option<bool> {
        // 对包含pump程序的交易感兴趣
        Some(tx.account_keys.contains(&solana_sdk::pubkey!(
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
        )))
    }

    async fn on_tx(self: Arc<Self>, tx: std::sync::Arc<TransactionFormat>) {
        // 更新最大slot
        {
            let mut max_slot = self.max_processed_slot.lock().unwrap();
            if tx.slot > *max_slot {
                *max_slot = tx.slot;
            }
        }

        // 将交易放入优先队列缓冲区
        let ordered_tx = OrderedTx {
            slot: tx.slot,
            index: tx.index,
            tx: tx.clone(),
        };

        self.tx_buffer.lock().unwrap().push(ordered_tx);

        // 增加入队计数
        let count = self.enqueued_count.fetch_add(1, AtomicOrdering::Relaxed) + 1;
        if count == 100 {
            info!("✅ Buffer filled: 100 transactions enqueued, processing will start");
        }
    }
}
