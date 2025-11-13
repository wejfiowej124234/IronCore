mod util;

#[cfg(feature = "test-env")]
use async_trait::async_trait;
#[cfg(feature = "test-env")]
use std::any::Any;
#[cfg(feature = "test-env")]
use std::sync::Arc;
#[cfg(feature = "test-env")]
use defi_hot_wallet::core::config::{BlockchainConfig, SecurityConfig, StorageConfig, WalletConfig};
#[cfg(feature = "test-env")]
use defi_hot_wallet::storage::{KeyLabelRecord, KeyVersionRecord, WalletMetadata, WalletStorage, WalletStorageTrait};
#[cfg(feature = "test-env")]
use defi_hot_wallet::blockchain::bridge::{BridgeTransaction, BridgeTransactionStatus};

// Build a minimal WalletConfig for tests (in-memory sqlite)
#[cfg(feature = "test-env")]
fn create_test_config() -> WalletConfig {
    WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(1),
            connection_timeout_seconds: Some(5),
        },
        blockchain: BlockchainConfig {
            networks: std::collections::HashMap::new(),
        },
        quantum_safe: false,
        multi_sig_threshold: 2,
        derivation: Default::default(),
        security: SecurityConfig::default(),
    }
}

// A delegating storage wrapper which fails on store_bridge_transaction
#[cfg(feature = "test-env")]
struct FailingStore {
    inner: Arc<WalletStorage>,
}

#[cfg(feature = "test-env")]
impl FailingStore {
    fn new(inner: Arc<WalletStorage>) -> Self {
        Self { inner }
    }
}

#[cfg(feature = "test-env")]
#[async_trait]
impl WalletStorageTrait for FailingStore {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn store_wallet(&self, name: &str, data: &[u8], quantum_safe: bool) -> anyhow::Result<()> {
        self.inner.store_wallet(name, data, quantum_safe).await
    }

    async fn load_wallet(&self, name: &str) -> anyhow::Result<(Vec<u8>, bool)> {
        self.inner.load_wallet(name).await
    }

    async fn list_wallets(&self) -> anyhow::Result<Vec<WalletMetadata>> {
        self.inner.list_wallets().await
    }

    async fn delete_wallet(&self, name: &str) -> anyhow::Result<()> {
        self.inner.delete_wallet(name).await
    }

    async fn update_wallet_encrypted_data(&self, name: &str, data: &[u8]) -> anyhow::Result<()> {
        self.inner.update_wallet_encrypted_data(name, data).await
    }

    async fn store_bridge_transaction(&self, _tx: &BridgeTransaction) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("injected store_bridge_transaction failure for test"))
    }

    async fn get_bridge_transaction(&self, id: &str) -> anyhow::Result<BridgeTransaction> {
        self.inner.get_bridge_transaction(id).await
    }

    async fn update_bridge_transaction_status(&self, id: &str, status: BridgeTransactionStatus, source_tx_hash: Option<String>) -> anyhow::Result<()> {
        self.inner.update_bridge_transaction_status(id, status, source_tx_hash).await
    }

    async fn rotation_upsert_label(&self, label: &str, current_version: i64, current_id: Option<&str>) -> anyhow::Result<()> {
        self.inner.rotation_upsert_label(label, current_version, current_id).await
    }

    async fn rotation_insert_version(&self, label: &str, version: i64, key_id: &str) -> anyhow::Result<()> {
        self.inner.rotation_insert_version(label, version, key_id).await
    }

    async fn rotation_mark_retired(&self, label: &str, version: i64) -> anyhow::Result<()> {
        self.inner.rotation_mark_retired(label, version).await
    }

    async fn rotation_inc_usage(&self, label: &str, version: i64) -> anyhow::Result<()> {
        self.inner.rotation_inc_usage(label, version).await
    }

    async fn rotation_get_label(&self, label: &str) -> anyhow::Result<Option<KeyLabelRecord>> {
        self.inner.rotation_get_label(label).await
    }

    async fn rotation_get_version(&self, label: &str, version: i64) -> anyhow::Result<Option<KeyVersionRecord>> {
        self.inner.rotation_get_version(label, version).await
    }

    async fn reserve_next_nonce(&self, network: &str, address: &str, initial: u64) -> anyhow::Result<u64> {
        self.inner.reserve_next_nonce(network, address, initial).await
    }

    async fn mark_nonce_used(&self, network: &str, address: &str, nonce: u64) -> anyhow::Result<()> {
        self.inner.mark_nonce_used(network, address, nonce).await
    }
}

#[tokio::test(flavor = "current_thread")]
#[cfg(feature = "test-env")]
async fn test_bridge_errors_when_persistence_fails() {
    // Create an inner real storage and wrap with failing stub
    let inner = Arc::new(WalletStorage::new_with_url("sqlite::memory:").await.expect("in-memory storage init"));
    let failing = Arc::new(FailingStore::new(inner.clone()));

    // Build config and a WalletManager using the failing storage
    let config = create_test_config();
    
    // Inject default test master key so create_wallet doesn't try to read env var
    let zeros: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let test_master_key = defi_hot_wallet::security::secret::vec_to_secret(zeros);
    
    #[cfg(feature = "test-env")]
    {
        defi_hot_wallet::core::wallet_manager::set_test_master_key_default(test_master_key);
    }
    
    let manager = defi_hot_wallet::core::wallet_manager::WalletManager::new_with_storage(config, failing.clone())
        .await
        .expect("manager init");
    // Set deterministic test envelope encryption key (base64 of 32 zero bytes)
    std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    // Enable mock bridge behaviour for tests
    std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");
    std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    std::env::set_var("BRIDGE_MOCK", "1");

    // Create a wallet using the manager; create_wallet delegates to storage and should succeed
    manager.create_wallet("persist_fail_wallet", false).await.expect("create wallet");

    // Now call bridge_assets; the storage.store_bridge_transaction is injected to fail,
    // and per strict persistence change the call should return an error.
    let res = manager.bridge_assets("persist_fail_wallet", "eth", "polygon", "USDC", "1").await;

    assert!(res.is_err(), "Expected bridge_assets to fail when persistence fails");
    let err = res.err().unwrap();
    let msg = format!("{}", err);
    assert!(msg.contains("Failed to persist bridge transaction") || msg.contains("Storage error"));
}