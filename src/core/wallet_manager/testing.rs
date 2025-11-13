//! 测试辅助模块
//!
//! 提供测试专用的辅助功能

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use crate::security::SecretVec;
use crate::core::{config::WalletConfig, errors::WalletError};
use super::WalletManager;

// 测试主密钥注入存储
static TEST_MASTER_KEYS: Lazy<Mutex<HashMap<String, SecretVec>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static TEST_MASTER_DEFAULT: Lazy<Mutex<Option<SecretVec>>> =
    Lazy::new(|| Mutex::new(None));

/// 为特定wallet注入测试主密钥
pub fn inject_test_master_key(name: &str, key: SecretVec) {
    let mut map = TEST_MASTER_KEYS.lock().unwrap();
    map.insert(name.to_string(), key);
}

/// 设置默认测试主密钥
pub fn set_test_master_key_default(key: SecretVec) {
    let mut def = TEST_MASTER_DEFAULT.lock().unwrap();
    *def = Some(key);
}

/// Clear所有注入的测试主密钥
pub fn clear_injected_test_master_keys() {
    TEST_MASTER_KEYS.lock().unwrap().clear();
    *TEST_MASTER_DEFAULT.lock().unwrap() = None;
}

/// fetch测试主密钥（如果有）
pub fn get_test_master_key(name: &str) -> Option<SecretVec> {
    let map = TEST_MASTER_KEYS.lock().unwrap();
    if let Some(key) = map.get(name) {
        return Some(key.clone());
    }
    
    let def = TEST_MASTER_DEFAULT.lock().unwrap();
    def.clone()
}

impl WalletManager {
    /// 测试专用构造函数（带自定义存储）
    ///
    /// # Arguments
    /// * `config` - wallet配置
    /// * `storage` - 自定义存储实现
    #[cfg(any(test, feature = "test-env"))]
    pub async fn new_with_storage(
        config: WalletConfig,
        _storage: Arc<dyn crate::storage::WalletStorageTrait>,
    ) -> Result<Self, WalletError> {
        use parking_lot::RwLock;
        use std::sync::Arc;
        use std::collections::HashMap;

        Ok(Self {
            config,
            wallets: Arc::new(RwLock::new(HashMap::new())),
            nonce_tracker: Arc::new(RwLock::new(HashMap::new())),
            bridge_transactions: Arc::new(RwLock::new(HashMap::new())),
            // Ethereum 暂时禁用
            // #[cfg(feature = "ethereum")]
            // ethereum_clients: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

