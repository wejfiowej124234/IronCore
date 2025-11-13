mod util;

#[cfg(feature = "test-env")]
use async_trait::async_trait;
#[cfg(feature = "test-env")]
use axum_test::TestServer;
#[cfg(feature = "test-env")]
use defi_hot_wallet::api::server::WalletServer;
#[cfg(feature = "test-env")]
use defi_hot_wallet::blockchain::traits::BlockchainClient;
#[cfg(feature = "test-env")]
use defi_hot_wallet::core::config::{StorageConfig, WalletConfig};
#[cfg(feature = "test-env")]
use defi_hot_wallet::core::errors::WalletError;
#[cfg(feature = "test-env")]
use defi_hot_wallet::network::rate_limit::RateLimiter;
#[cfg(feature = "test-env")]
use serde_json::json;
#[cfg(feature = "test-env")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "test-env")]
use std::sync::Arc;
#[cfg(feature = "test-env")]
use std::time::Duration;

#[cfg(feature = "test-env")]
struct MockClient {
    chain_nonce: Arc<AtomicU64>,
}

#[cfg(feature = "test-env")]
#[async_trait]
impl BlockchainClient for MockClient {
    fn clone_box(&self) -> Box<dyn BlockchainClient> {
        Box::new(MockClient { chain_nonce: Arc::clone(&self.chain_nonce) })
    }

    async fn get_balance(&self, _address: &str) -> Result<String, WalletError> {
        Ok("0".to_string())
    }

    async fn send_transaction(
        &self,
        _private_key: &defi_hot_wallet::core::domain::PrivateKey,
        _to_address: &str,
        _amount: &str,
    ) -> Result<String, WalletError> {
        // Simulate on-chain nonce increment when a tx is sent
        self.chain_nonce.fetch_add(1, Ordering::SeqCst);
        Ok("0xmocktxhash".to_string())
    }

    async fn get_transaction_status(
        &self,
        _tx_hash: &str,
    ) -> Result<defi_hot_wallet::blockchain::traits::TransactionStatus, WalletError> {
        Err(WalletError::Other("not implemented".to_string()))
    }

    async fn estimate_fee(&self, _to_address: &str, _amount: &str) -> Result<String, WalletError> {
        Ok("0".to_string())
    }

    async fn get_nonce(&self, _address: &str) -> Result<u64, WalletError> {
        Ok(self.chain_nonce.load(Ordering::SeqCst))
    }

    async fn get_block_number(&self) -> Result<u64, WalletError> {
        Ok(0)
    }

    fn validate_address(&self, _address: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn get_network_name(&self) -> &str {
        "mock"
    }
    fn get_native_token(&self) -> &str {
        "MOCK"
    }
}

#[tokio::test(flavor = "current_thread")]
#[cfg(feature = "test-env")]
async fn test_send_route_concurrent() {
    // Ensure deterministic test KEK and test flags for wallet creation
    util::set_test_env();
    let config = WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    let test_master_key = defi_hot_wallet::security::secret::vec_to_secret(
        std::iter::repeat_n(0u8, 32).collect::<Vec<u8>>(),
    );
    
    // Inject test master key using test-env feature
    #[cfg(feature = "test-env")]
    {
        defi_hot_wallet::core::wallet_manager::set_test_master_key_default(test_master_key.clone());
    }
    
    // Build a WalletManager with in-memory storage so we can inject mock clients
    let storage = Arc::new(
        defi_hot_wallet::storage::WalletStorage::new_with_url("sqlite::memory:")
            .await
            .expect("storage init"),
    );
    let mut wm = defi_hot_wallet::core::wallet_manager::WalletManager::new_with_storage(
        config.clone(),
        storage,
    )
    .await
    .expect("wm init");

    // Note: blockchain_clients field was removed from WalletManager
    // Tests will use default blockchain client behavior
    // let mut clients = std::collections::HashMap::new();
    // clients.insert(
    //     "eth".to_string(),
    //     Box::new(MockClient { chain_nonce: Arc::new(AtomicU64::new(200)) })
    //         as Box<dyn BlockchainClient>,
    // );
    // wm.blockchain_clients = Arc::new(clients);

    // Keep an Arc to the wallet manager so we can inspect state after requests
    let wm_arc = Arc::new(wm);

    // Construct a test WalletServer using the prepared WalletManager
    let server = WalletServer {
        wallet_manager: wm_arc.clone(),
        host: "127.0.0.1".to_string(),
        port: 0,
        config,
        api_key: None,
        rate_limiter: Arc::new(RateLimiter::new(10000, Duration::from_secs(1))),
    };

    let test_server = TestServer::new(server.create_router().await).unwrap();

    // Create wallet
    let wallet_name = "http_concurrent_test";
    let create_res = test_server
        .post("/api/wallets")
        .json(&json!({ "name": wallet_name, "quantum_safe": false }))
        .await;
    create_res.assert_status_ok();

    // Prepare request body
    // Use raw JSON body to avoid needing Serialize for internal types
    // Use a valid-looking Ethereum address (40 hex chars)
    let req_json = json!({ "to_address": "0x1111111111111111111111111111111111111111", "amount": "1", "network": "eth" });

    // Fire 4 concurrent sends
    use futures::future::join_all;
    let srv = Arc::new(test_server);
    let futs: Vec<_> = (0..4)
        .map(|_| {
            let s = srv.clone();
            let body = req_json.clone();
            async move { s.post(&format!("/api/wallets/{}/send", wallet_name)).json(&body).await }
        })
        .collect();

    let res = join_all(futs).await;
    for r in res {
        r.assert_status_ok();
    }

    // After sends, check that next nonce advanced (call via WalletManager directly)
    let next_nonce = wm_arc.get_next_nonce("0xrecipient00", "eth").await.unwrap();
    assert_eq!(next_nonce, 200 + 4);
}
