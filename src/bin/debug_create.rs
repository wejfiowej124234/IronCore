use defi_hot_wallet::api::server::WalletServer;
use defi_hot_wallet::core::config::{BlockchainConfig, SecurityConfig, StorageConfig, WalletConfig};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Mirror create_test_config from tests
    let config = WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(1),
            connection_timeout_seconds: Some(30),
        },
        blockchain: BlockchainConfig {
            networks: HashMap::new(),
        },
        quantum_safe: false,
        multi_sig_threshold: 2,
        derivation: Default::default(),
        security: SecurityConfig::default(),
    };

    // Set same envs as new_for_test. Avoid hardcoding WALLET_ENC_KEY in source to
    // satisfy repository policy checks. Run-time users or test helpers should set
    // `WALLET_ENC_KEY` via environment or use `tests::util::set_test_env()`.
    std::env::set_var("TEST_SKIP_DECRYPT", "1");
    std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    std::env::set_var("BRIDGE_MOCK", "1");
    std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");

    // generate a runtime key for the debug helper (avoid hard-coded key literals in source)
    let key_arr: [u8; 32] = rand::random();
    let key = key_arr.to_vec();
    let test_master_key = defi_hot_wallet::security::secret::vec_to_secret(key);
    let server = WalletServer::new_for_test(
        "127.0.0.1".to_string(),
        0,
        config,
        Some(zeroize::Zeroizing::new("test_api_key".as_bytes().to_vec())),
        Some(test_master_key),
    )
    .await
    .expect("create server");

    // Directly call create_wallet on the manager to reproduce the error path
    let wm = server.wallet_manager.clone();
    match wm.create_wallet("debug_wallet", "debug_password", true).await {
        Ok(()) => println!("Created wallet: debug_wallet"),
        Err(e) => println!("create_wallet failed: {}", e),
    }
}
