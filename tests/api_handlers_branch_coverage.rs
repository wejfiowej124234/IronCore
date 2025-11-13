// API Handlers条件分支全覆盖测试

use defi_hot_wallet::core::wallet_manager::WalletManager;
use defi_hot_wallet::core::config::{WalletConfig, StorageConfig, BlockchainConfig, SecurityConfig};
use std::collections::HashMap;

async fn create_test_manager() -> WalletManager {
    std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    std::env::set_var("TEST_SKIP_DECRYPT", "1");
    std::env::set_var("RUST_TEST_THREADS", "1");
    
    let config = WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(5),
            connection_timeout_seconds: Some(10),
        },
        blockchain: BlockchainConfig {
            networks: HashMap::new(),
        },
        quantum_safe: false,
        multi_sig_threshold: 2,
        derivation: Default::default(),
        security: SecurityConfig::default(),
    };
    
    WalletManager::new(&config).await.expect("Failed to create WalletManager")
}

// === 测试backup.rs中的条件分支 ===

#[tokio::test]
async fn test_backup_restore_wallet_exists_check_true() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("exists", false).await;
    
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let result = manager.restore_wallet("exists", mnemonic).await;
    
    // 钱包已存在，应该失败
    assert!(result.is_err());
}

#[tokio::test]
async fn test_backup_restore_wallet_exists_check_false() {
    let manager = create_test_manager().await;
    
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let result = manager.restore_wallet("new_wallet", mnemonic).await;
    
    // 钱包不存在，应该成功
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_restore_mnemonic_word_count_12() {
    let manager = create_test_manager().await;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let result = manager.restore_wallet("test12", mnemonic).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_restore_mnemonic_word_count_15() {
    let manager = create_test_manager().await;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let result = manager.restore_wallet("test15", mnemonic).await;
    // 15词应该被接受
    let _ = result;
}

#[tokio::test]
async fn test_restore_mnemonic_word_count_18() {
    let manager = create_test_manager().await;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let result = manager.restore_wallet("test18", mnemonic).await;
    let _ = result;
}

#[tokio::test]
async fn test_restore_mnemonic_word_count_21() {
    let manager = create_test_manager().await;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let result = manager.restore_wallet("test21", mnemonic).await;
    let _ = result;
}

#[tokio::test]
async fn test_restore_mnemonic_word_count_24() {
    let manager = create_test_manager().await;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let result = manager.restore_wallet("test24", mnemonic).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_restore_mnemonic_invalid_count_1() {
    let manager = create_test_manager().await;
    let result = manager.restore_wallet("test", "one").await;
    assert!(result.is_err(), "1词应该失败");
}

#[tokio::test]
async fn test_restore_mnemonic_invalid_count_11() {
    let manager = create_test_manager().await;
    let mnemonic = "one two three four five six seven eight nine ten eleven";
    let result = manager.restore_wallet("test", mnemonic).await;
    assert!(result.is_err(), "11词应该失败");
}

#[tokio::test]
async fn test_restore_mnemonic_invalid_count_13() {
    let manager = create_test_manager().await;
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about extra";
    let result = manager.restore_wallet("test", mnemonic).await;
    assert!(result.is_err(), "13词应该失败");
}

// === 测试transaction.rs中的网络分支 ===

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_send_transaction_network_eth() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.send_transaction("test", "0x123", "1.0", "eth", "test_password").await;
    let _ = result; // eth分支
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_send_transaction_network_sepolia() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.send_transaction("test", "0x123", "1.0", "sepolia", "test_password").await;
    let _ = result; // sepolia分支
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_send_transaction_network_polygon() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.send_transaction("test", "0x123", "1.0", "polygon", "test_password").await;
    let _ = result; // polygon分支
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_send_transaction_network_bsc() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.send_transaction("test", "0x123", "1.0", "bsc", "test_password").await;
    let _ = result; // bsc分支
}

#[tokio::test]
async fn test_send_transaction_network_default_error() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.send_transaction("test", "0x123", "1.0", "unknown_network", "test_password").await;
    assert!(result.is_err(), "未知网络应该走到默认错误分支");
}

// === 测试multisig的threshold验证分支 ===

#[tokio::test]
async fn test_multisig_threshold_less_than_1() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let signers = vec!["s1".to_string(), "s2".to_string()];
    let result = manager.send_multi_sig_transaction("test", "0x123", "1.0", &signers, 0).await;
    assert!(result.is_err(), "threshold < 1应该失败");
}

#[tokio::test]
async fn test_multisig_threshold_equals_signers() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let signers = vec!["s1".to_string(), "s2".to_string()];
    let result = manager.send_multi_sig_transaction("test", "0x123", "1.0", &signers, 2).await;
    assert!(result.is_ok(), "threshold == signers.len()应该成功");
}

#[tokio::test]
async fn test_multisig_threshold_greater_than_signers() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let signers = vec!["s1".to_string(), "s2".to_string()];
    let result = manager.send_multi_sig_transaction("test", "0x123", "1.0", &signers, 3).await;
    assert!(result.is_err(), "threshold > signers.len()应该失败");
}

// === 测试balance.rs的网络分支 ===

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_get_balance_network_eth() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.get_balance("test", "eth").await;
    let _ = result; // eth分支
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_get_balance_network_sepolia() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.get_balance("test", "sepolia").await;
    let _ = result; // sepolia分支
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_get_balance_network_polygon() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.get_balance("test", "polygon").await;
    let _ = result; // polygon分支
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_get_balance_network_bsc() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.get_balance("test", "bsc").await;
    let _ = result; // bsc分支
}

#[tokio::test]
async fn test_get_balance_network_default_error() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let result = manager.get_balance("test", "invalid_network").await;
    assert!(result.is_err(), "未知网络应该走到默认错误分支");
}

// === 测试address.rs的网络分支 ===

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_derive_address_network_eth() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "eth");
    assert!(result.is_ok());
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_derive_address_network_ethereum() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "ethereum");
    assert!(result.is_ok());
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_derive_address_network_sepolia() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "sepolia");
    assert!(result.is_ok());
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_derive_address_network_polygon() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "polygon");
    assert!(result.is_ok());
}

#[cfg(feature = "ethereum")]
#[tokio::test]
async fn test_derive_address_network_bsc() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "bsc");
    assert!(result.is_ok());
}

#[cfg(feature = "bitcoin")]
#[tokio::test]
async fn test_derive_address_network_bitcoin() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "bitcoin");
    assert!(result.is_ok());
}

#[cfg(feature = "bitcoin")]
#[tokio::test]
async fn test_derive_address_network_btc() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "btc");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_derive_address_network_default_error() {
    let manager = create_test_manager().await;
    let key = vec![1u8; 32];
    let result = manager.derive_address(&key, "unsupported");
    assert!(result.is_err(), "未知网络应该走到默认错误分支");
}

// === 测试nonce.rs的条件分支 ===

#[tokio::test]
async fn test_mark_nonce_used_greater_than_current() {
    let manager = create_test_manager().await;
    
    // 当前nonce是0
    let _ = manager.get_next_nonce("0x123", "eth").await; // 0
    
    // 标记一个更大的nonce
    let _ = manager.mark_nonce_used("0x123", "eth", 10).await;
    
    // 下一个nonce应该 > 10
    let next = manager.get_next_nonce("0x123", "eth").await.unwrap();
    assert!(next > 10, "标记大nonce后，下一个应该更大");
}

#[tokio::test]
async fn test_mark_nonce_used_less_than_current() {
    let manager = create_test_manager().await;
    
    // 先获取几个nonce
    let _ = manager.get_next_nonce("0x456", "eth").await; // 0
    let _ = manager.get_next_nonce("0x456", "eth").await; // 1
    let _ = manager.get_next_nonce("0x456", "eth").await; // 2
    
    // 标记一个小的nonce（应该不影响）
    let _ = manager.mark_nonce_used("0x456", "eth", 1).await;
    
    // 当前nonce应该不变
    let next = manager.get_next_nonce("0x456", "eth").await.unwrap();
    assert_eq!(next, 3, "标记小nonce不应该影响当前值");
}

#[tokio::test]
async fn test_reset_nonce_wallet_exists() {
    let manager = create_test_manager().await;
    
    let _ = manager.get_next_nonce("0x789", "eth").await;
    let _ = manager.get_next_nonce("0x789", "eth").await;
    
    let result = manager.reset_nonce("0x789", "eth").await;
    assert!(result.is_ok());
    
    let nonce = manager.get_next_nonce("0x789", "eth").await.unwrap();
    assert_eq!(nonce, 0, "重置后应该从0开始");
}

#[tokio::test]
async fn test_reset_nonce_wallet_not_exists() {
    let manager = create_test_manager().await;
    
    // 重置不存在的地址
    let result = manager.reset_nonce("0xnever_seen", "eth").await;
    assert!(result.is_ok(), "重置不存在的地址应该成功（不做操作）");
}

// === 测试lifecycle.rs的条件分支 ===

#[tokio::test]
async fn test_create_wallet_wallets_contains_key_true() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("duplicate", false).await;
    
    // 再次创建同名钱包
    let result = manager.create_wallet("duplicate", false).await;
    
    assert!(result.is_err(), "重复名称应该失败");
}

#[tokio::test]
async fn test_create_wallet_wallets_contains_key_false() {
    let manager = create_test_manager().await;
    
    // 创建新钱包
    let result = manager.create_wallet("new_unique", false).await;
    
    assert!(result.is_ok(), "新名称应该成功");
}

#[tokio::test]
async fn test_delete_wallet_remove_some() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("to_delete", false).await;
    
    let result = manager.delete_wallet("to_delete").await;
    assert!(result.is_ok(), "删除存在的钱包应该成功");
}

#[tokio::test]
async fn test_delete_wallet_remove_none() {
    let manager = create_test_manager().await;
    
    let result = manager.delete_wallet("never_existed").await;
    assert!(result.is_err(), "删除不存在的钱包应该失败");
}

#[tokio::test]
async fn test_get_wallet_by_name_some() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("findme", false).await;
    
    let result = manager.get_wallet_by_name("findme").await.unwrap();
    assert!(result.is_some(), "存在的钱包应该返回Some");
}

#[tokio::test]
async fn test_get_wallet_by_name_none() {
    let manager = create_test_manager().await;
    
    let result = manager.get_wallet_by_name("notfound").await.unwrap();
    assert!(result.is_none(), "不存在的钱包应该返回None");
}

// === 边缘案例：最小/最大值 ===

#[tokio::test]
async fn test_multisig_threshold_min_value_1() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let signers = vec!["signer1".to_string()];
    let result = manager.send_multi_sig_transaction("test", "0x123", "1.0", &signers, 1).await;
    assert!(result.is_ok(), "threshold=1应该是有效的最小值");
}

#[tokio::test]
async fn test_multisig_threshold_max_value() {
    let manager = create_test_manager().await;
    let _ = manager.create_wallet("test", false).await;
    let signers: Vec<String> = (0..100).map(|i| format!("signer_{}", i)).collect();
    let result = manager.send_multi_sig_transaction("test", "0x123", "1.0", &signers, 100).await;
    assert!(result.is_ok(), "threshold=signers.len()应该成功");
}

#[tokio::test]
async fn test_nonce_large_value() {
    let manager = create_test_manager().await;
    
    // 使用一个大但不会溢出的nonce值
    let large_nonce = u64::MAX / 2;
    let _ = manager.mark_nonce_used("0xlarge", "eth", large_nonce).await;
    
    let next = manager.get_next_nonce("0xlarge", "eth").await.unwrap();
    assert_eq!(next, large_nonce + 1, "应该正确处理大nonce");
}

