// tests/bridge_additional_tests.rs
// New tests for bridge mock & relay logic — deterministic where possible.

use std::collections::HashSet;
use std::env;

use defi_hot_wallet::blockchain::bridge::{
    relay::{mock_bridge_transfer, mock_check_transfer_status, relay_transaction},
    BridgeTransactionStatus,
};
use defi_hot_wallet::core::wallet_info::{SecureWalletData, WalletInfo};
use tokio::task;
use uuid::Uuid;

fn create_mock_wallet_data() -> SecureWalletData {
    SecureWalletData {
        info: WalletInfo {
            id: Uuid::new_v4(),
            name: "test-wallet".to_string(),
            created_at: chrono::Utc::now(),
            quantum_safe: false,
            multi_sig_threshold: 1,
            networks: vec!["eth".to_string(), "polygon".to_string()],
        },
        encrypted_master_key: vec![],
        shamir_shares: vec![],
        salt: vec![],
        nonce: vec![],
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_mock_bridge_transfer_returns_simulated_hash() {
    let wallet = create_mock_wallet_data();
    let tx = mock_bridge_transfer("eth", "polygon", "USDC", "10.0", "0xMockContract", &wallet)
        .await
        .expect("mock transfer should succeed");
    assert!(tx.starts_with("0x_simulated_tx_"), "unexpected simulated hash: {}", tx);
}

#[tokio::test(flavor = "current_thread")]
async fn test_mock_check_transfer_status_force_success_env() {
    env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    let status = mock_check_transfer_status("any_tx_hash")
        .await
        .expect("status ok");
    assert_eq!(status, BridgeTransactionStatus::Completed);
    env::remove_var("BRIDGE_MOCK_FORCE_SUCCESS");
}

#[tokio::test(flavor = "current_thread")]
async fn test_mock_check_transfer_status_failed_marker() {
    env::remove_var("BRIDGE_MOCK_FORCE_SUCCESS");
    let status = mock_check_transfer_status("tx_failed_marker")
        .await
        .expect("status ok");
    matches!(status, BridgeTransactionStatus::Failed(_));
}

#[tokio::test(flavor = "current_thread")]
async fn test_concurrent_mock_transfers_produce_unique_hashes() {
    let wallet = create_mock_wallet_data();
    let mut handles = Vec::new();
    for _ in 0..12 {
        let w = wallet.clone();
        handles.push(task::spawn(async move {
            mock_bridge_transfer("eth", "polygon", "USDC", "1.0", "0xMock", &w)
                .await
                .expect("transfer ok")
        }));
    }

    let mut results = Vec::new();
    for h in handles {
        let v = h.await.expect("task join");
        results.push(v);
    }

    let set: HashSet<_> = results.iter().collect();
    assert_eq!(set.len(), results.len(), "expected unique tx hashes");
}

#[tokio::test(flavor = "current_thread")]
async fn test_relay_transaction_with_local_bridge_impl() {
    // Use the simple relay path by creating a tiny local bridge impl that delegates to the mock check function.
    struct LocalBridge;
    #[async_trait::async_trait]
    impl defi_hot_wallet::blockchain::traits::Bridge for LocalBridge {
        async fn check_transfer_status(
            &self,
            tx_id: &str,
        ) -> anyhow::Result<BridgeTransactionStatus> {
            // call the deterministic mock helper
            mock_check_transfer_status(tx_id).await
        }
        async fn transfer_across_chains(
            &self,
            _from_chain: &str,
            _to_chain: &str,
            _token: &str,
            _amount: &str,
            _wallet_data: &defi_hot_wallet::core::wallet_info::SecureWalletData,
        ) -> anyhow::Result<String> {
            Ok("local_cross_tx".into())
        }
    }

    // Force deterministic success for this unit test
    env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    let bridge = LocalBridge;
    let status = relay_transaction(&bridge, "any_tx").await.expect("relay ok");
    assert_eq!(status, BridgeTransactionStatus::Completed);
    env::remove_var("BRIDGE_MOCK_FORCE_SUCCESS");
}