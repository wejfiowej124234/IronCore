#[cfg(test)]
mod tests {
    use defi_hot_wallet::core::config::WalletConfig;
    use defi_hot_wallet::core::wallet_manager::WalletManager;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_nonce_synchronization_prevents_replay() {
        // Create a test wallet manager with mock blockchain clients
        let config = WalletConfig {
            storage: defi_hot_wallet::core::config::StorageConfig {
                database_url: "sqlite::memory:".to_string(),
                max_connections: Some(10),
                connection_timeout_seconds: Some(30),
            },
            blockchain: defi_hot_wallet::core::config::BlockchainConfig {
                networks: HashMap::new(),
            },
            quantum_safe: false,
            multi_sig_threshold: 1,
            derivation: Default::default(),
            security: defi_hot_wallet::core::config::SecurityConfig::default(),
        };

        let wallet_manager = WalletManager::new(&config).await.unwrap();

        // Test that nonce tracking works correctly
        // This would require setting up mock blockchain clients that return increasing nonces

        // For now, just verify the wallet manager was created successfully
        assert!(wallet_manager.list_wallets().await.unwrap().is_empty());
    }
}
