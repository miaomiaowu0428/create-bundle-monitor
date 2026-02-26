use heed::types::{Bytes, SerdeBincode};
use heed::{Database, Env, EnvOpenOptions};
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

/// 存储在数据库中的bundle数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxBundle {
    pub mint: Pubkey,
    pub create_tx: TxInfo,
    pub follow_txs: Vec<TxInfo>, // 最多4笔，可以少
}

/// Bundle存储管理器 (基于LMDB，支持多进程读)
pub struct BundleStore {
    env: Env,
    db: Database<Bytes, SerdeBincode<TxBundle>>,
}

impl BundleStore {
    /// 创建或打开一个bundle存储
    pub fn open(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 创建目录（如果不存在）
        std::fs::create_dir_all(db_path)?;

        // 打开环境，设置10GB map size，支持多进程读
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(10 * 1024 * 1024 * 1024) // 10 GB
                .max_dbs(1)
                .open(db_path)?
        };

        // 创建或打开数据库（需要写事务）
        let mut wtxn = env.write_txn()?;
        let db = env.create_database(&mut wtxn, Some("bundles"))?;
        wtxn.commit()?;

        Ok(Self { env, db })
    }

    /// 存储一个bundle
    pub fn store(&self, bundle: &TxBundle) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtxn = self.env.write_txn()?;
        let key = bundle.mint.to_bytes();
        self.db.put(&mut wtxn, &key, bundle)?;
        wtxn.commit()?;
        Ok(())
    }

    /// 获取指定mint的bundle
    pub fn get(&self, mint: &Pubkey) -> Result<Option<TxBundle>, Box<dyn std::error::Error>> {
        let rtxn = self.env.read_txn()?;
        let key = mint.to_bytes();
        Ok(self.db.get(&rtxn, &key)?)
    }

    /// 列出所有已存储的mint
    pub fn list_mints(&self) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
        let rtxn = self.env.read_txn()?;
        let mut mints = Vec::new();

        for item in self.db.iter(&rtxn)? {
            let (key, _) = item?;
            if key.len() == 32 {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(key);
                mints.push(Pubkey::new_from_array(bytes));
            }
        }

        Ok(mints)
    }

    /// 获取所有bundles
    pub fn list_all(&self) -> Result<Vec<TxBundle>, Box<dyn std::error::Error>> {
        let rtxn = self.env.read_txn()?;
        let mut bundles = Vec::new();

        for item in self.db.iter(&rtxn)? {
            let (_, bundle) = item?;
            bundles.push(bundle);
        }

        Ok(bundles)
    }

    /// 获取数据库中的bundle数量
    pub fn count(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let rtxn = self.env.read_txn()?;
        Ok(self.db.len(&rtxn)? as usize)
    }

    /// 删除指定mint的bundle
    pub fn remove(&self, mint: &Pubkey) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtxn = self.env.write_txn()?;
        let key = mint.to_bytes();
        self.db.delete(&mut wtxn, &key)?;
        wtxn.commit()?;
        Ok(())
    }

    /// 清空所有数据
    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtxn = self.env.write_txn()?;
        self.db.clear(&mut wtxn)?;
        wtxn.commit()?;
        Ok(())
    }
}
