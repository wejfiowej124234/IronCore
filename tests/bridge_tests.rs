// tests/bridge_tests.rs
//! 妗ユ帴鍔熻兘娴嬭瘯

mod util;

use anyhow::Result;
use chrono::Utc;
use defi_hot_wallet::blockchain::bridge::{
    Bridge, BridgeTransactionStatus, EthereumToBSCBridge,
    PolygonToEthereumBridge,
};
use defi_hot_wallet::core::wallet_info::{SecureWalletData, WalletInfo};
use std::str::FromStr;
use uuid::Uuid;

fn create_mock_wallet_data() -> SecureWalletData {
    SecureWalletData {
        info: WalletInfo {
            id: Uuid::from_str("12345678-1234-1234-1234-123456789012").unwrap(),
            name: "test-wallet".to_string(),
            created_at: Utc::now(),
            quantum_safe: true,
            multi_sig_threshold: 1,
            networks: vec!["eth".to_string(), "polygon".to_string()],
        },
        encrypted_master_key: vec![1, 2, 3, 4],
        shamir_shares: vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]],
        salt: vec![5, 6, 7, 8],
        nonce: vec![9, 10, 11, 12],
        schema_version: defi_hot_wallet::core::SecureWalletData::default_schema_version(),
        kek_id: None,
    }
}


#[tokio::test]
async fn test_ethereum_to_bsc_bridge() -> Result<()> {
    // Set mock behavior and centralized test env (KEEP BRIDGE_MOCK_FORCE_SUCCESS local)
    std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    util::set_test_env();

    let bridge = EthereumToBSCBridge::new("0xMockEthBscBridge");
    let wallet_data = create_mock_wallet_data();

    bridge.transfer_across_chains("eth", "bsc", "USDT", "75.0", &wallet_data).await?;
    Ok(())
}

#[tokio::test]
async fn integration_transfer_and_failed_marker() -> Result<()> {
    // Set mock behavior and centralized test env (KEEP BRIDGE_MOCK_FORCE_SUCCESS local)
    std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    util::set_test_env();

    let bridge = EthereumToBSCBridge::new("0xBridge");
    let w = create_mock_wallet_data();

    let tx = bridge.transfer_across_chains("eth", "bsc", "USDC", "1.0", &w).await?;
    assert!(tx.starts_with("0x_simulated_lock_tx_"));

    // explicit failed marker forces Failed status
    let failed_tx = "0x_marked_failed_tx";
    let status = bridge.check_transfer_status(failed_tx).await?;
    assert_eq!(
        status,
        BridgeTransactionStatus::Failed("Transaction explicitly marked as failed".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn integration_mock_bridge_variants_and_concurrent() -> Result<()> {
    // Set mock environment for bridge tests
    std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    // Allow bridge mocks in this test process
    std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");

    let e2b = EthereumToBSCBridge::new("0xE2B");
    let poly = PolygonToEthereumBridge::new("0xP2E");
    let w = create_mock_wallet_data();

    let t2 = e2b.transfer_across_chains("eth", "bsc", "USDT", "2.0", &w).await?;
    assert!(t2.starts_with("0x_simulated_tx_"));

    let t3 = poly.transfer_across_chains("polygon", "eth", "DAI", "3.0", &w).await?;
    assert!(t3.starts_with("0x_simulated_tx_"));

    // concurrent transfers should all succeed
    let handles = vec![
        tokio::spawn({
            let e2b = EthereumToBSCBridge::new("0xE2B");
            let w = create_mock_wallet_data();
            async move { e2b.transfer_across_chains("eth", "bsc", "USDT", "2.0", &w).await }
        }),
        tokio::spawn({
            let poly = PolygonToEthereumBridge::new("0xP2E");
            let w = create_mock_wallet_data();
            async move { poly.transfer_across_chains("polygon", "eth", "DAI", "3.0", &w).await }
        }),
    ];

    let results = futures::future::join_all(handles).await;
    for r in results {
        let ok = r.expect("task panicked")?;
        assert!(ok.starts_with("0x_simulated_tx_"));
    }

    Ok(())
}
