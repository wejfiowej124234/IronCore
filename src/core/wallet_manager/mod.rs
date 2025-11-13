//! Wallet Manager Core Module
//!
//! Provides complete lifecycle management for wallets, including creation, deletion, transactions, backup, etc.
//! 
//! ## Module Structure
//! - `lifecycle` - Wallet lifecycle management (create, delete, list)
//! - `keys` - Key management (generation, derivation, rotation)
//! - `transactions` - Transaction operations (send, multi-sig)
//! - `balance` - Balance queries
//! - `backup` - Backup and recovery
//! - `bridge` - Cross-chain bridging
//! - `nonce` - Nonce management
//! - `address` - Address derivation
//! - `testing` - Testing utilities

// Submodule declarations
pub mod lifecycle;      // Wallet lifecycle (create, delete, list)
pub mod keys;           // Key management (generation, derivation, rotation)
pub mod transactions;   // Transaction operations (send, multi-sig)
pub mod balance;        // Balance queries
pub mod backup;         // Backup and recovery
pub mod bridge;         // Cross-chain bridging
pub mod nonce;          // Nonce management
pub mod address;        // Address derivation
pub mod derivation;     // BIP44 address derivation (mnemonic version)
pub mod master_key_derivation;  // Master key address derivation (core algorithm)
pub mod signing;        // Transaction signing (Ethereum)
pub mod bitcoin_utxo;   // Bitcoin UTXO management
pub mod bitcoin_signing; // Bitcoin transaction signing
pub mod tx_history;     // Transaction history queries

// Testing utilities module
#[cfg(any(test, feature = "test-env"))]
pub mod testing;

// Re-export testing utility functions
#[cfg(any(test, feature = "test-env"))]
pub use testing::{
    inject_test_master_key,
    set_test_master_key_default,
    clear_injected_test_master_keys,
    get_test_master_key,
};
// Note: new_with_storage is a method on WalletManager, not a standalone function

// Core dependencies
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;

use crate::blockchain::{
    bridge::BridgeTransaction,
};
use crate::core::{
    config::WalletConfig,
    errors::WalletError,
    wallet_info::SecureWalletData,
};

// Ethereum temporarily disabled (dependency conflicts)
// #[cfg(feature = "ethereum")]
// use crate::blockchain::ethereum::EthereumClient;

// #[cfg(feature = "polygon")]

/// wallet管理器
/// 
/// 负责管理所有wallet的生命周期、transaction、备份等功能
pub struct WalletManager {
    /// wallet配置
    pub config: WalletConfig,
    
    /// wallet存储（名称 → wallet数据）
    pub wallets: Arc<RwLock<HashMap<String, SecureWalletData>>>,
    
    /// Nonce 追踪器（address → network → nonce）
    pub nonce_tracker: Arc<RwLock<HashMap<String, HashMap<String, u64>>>>,
    
    /// 桥接transaction存储
    pub bridge_transactions: Arc<RwLock<HashMap<String, BridgeTransaction>>>,
    
    // Ethereum 客户端缓存 (暂时禁用)
    // #[cfg(feature = "ethereum")]
    // pub ethereum_clients: Arc<RwLock<HashMap<String, Arc<EthereumClient>>>>,
    
    // #[cfg(feature = "polygon")]
}

impl WalletManager {
    /// 创建新的wallet管理器
    ///
    /// # Arguments
    /// * `config` - wallet配置
    ///
    /// # Returns
    /// 新的 WalletManager 实例
    pub async fn new(config: &WalletConfig) -> Result<Self, WalletError> {
        // validate数据库URL格式
        let db_url = &config.storage.database_url;
        if db_url.is_empty() {
            return Err(WalletError::ConfigError("Database URL cannot be empty".into()));
        }
        
        // check是否是支持的协议
        if !db_url.starts_with("sqlite:") && !db_url.starts_with("postgres:") && !db_url.starts_with("mysql:") {
            return Err(WalletError::ConfigError(format!("Unsupported database protocol in URL: {}", db_url)));
        }
        
        Ok(Self {
            config: config.clone(),
            wallets: Arc::new(RwLock::new(HashMap::new())),
            nonce_tracker: Arc::new(RwLock::new(HashMap::new())),
            bridge_transactions: Arc::new(RwLock::new(HashMap::new())),
            // Ethereum 暂时禁用
            // #[cfg(feature = "ethereum")]
            // ethereum_clients: Arc::new(RwLock::new(HashMap::new())),
            // #[cfg(feature = "polygon")]
        })
    }
}


