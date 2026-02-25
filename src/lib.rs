use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use utils::IndexedInstruction;

pub mod monitor;

/// 交易信息 - 只存储关键数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInfo {
    pub signature: Signature,
    pub slot: u64,
    pub index: u64,
    pub account_keys: Vec<Pubkey>,
    pub flattened_ixs: Vec<IndexedInstruction>,
}

/// 存储在sled中的bundle数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxBundle {
    pub mint: Pubkey,
    pub create_tx: TxInfo,
    pub follow_txs: Vec<TxInfo>, // 最多4笔，可以少
}

/// Bundle存储管理器
pub struct BundleStore {
    db: sled::Db,
}

impl BundleStore {
    /// 创建或打开一个bundle存储
    pub fn open(db_path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(db_path)?;
        Ok(Self { db })
    }

    /// 存储一个bundle
    pub fn store(&self, bundle: &TxBundle) -> Result<(), Box<dyn std::error::Error>> {
        let key = bundle.mint.to_bytes();
        let value = bincode::serialize(bundle)?;
        self.db.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    /// 获取指定mint的bundle
    pub fn get(&self, mint: &Pubkey) -> Result<Option<TxBundle>, Box<dyn std::error::Error>> {
        if let Some(data) = self.db.get(mint.to_bytes())? {
            let bundle: TxBundle = bincode::deserialize(&data)?;
            Ok(Some(bundle))
        } else {
            Ok(None)
        }
    }

    /// 列出所有已存储的mint
    pub fn list_mints(&self) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
        let mut mints = Vec::new();
        for item in self.db.iter() {
            let (key, _) = item?;
            if key.len() == 32 {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&key);
                mints.push(Pubkey::new_from_array(bytes));
            }
        }
        Ok(mints)
    }

    /// 获取所有bundles
    pub fn list_all(&self) -> Result<Vec<TxBundle>, Box<dyn std::error::Error>> {
        let mut bundles = Vec::new();
        for item in self.db.iter() {
            let (_, value) = item?;
            let bundle: TxBundle = bincode::deserialize(&value)?;
            bundles.push(bundle);
        }
        Ok(bundles)
    }

    /// 获取数据库中的bundle数量
    pub fn count(&self) -> usize {
        self.db.len()
    }

    /// 删除指定mint的bundle
    pub fn remove(&self, mint: &Pubkey) -> Result<(), Box<dyn std::error::Error>> {
        self.db.remove(mint.to_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    /// 清空所有数据
    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.db.clear()?;
        self.db.flush()?;
        Ok(())
    }

    /// 获取底层数据库引用（供monitor使用）
    pub fn db(&self) -> &sled::Db {
        &self.db
    }
}
