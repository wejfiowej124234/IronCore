// 阶段 1 第2部分：更多 0% 覆盖率模块测试

// ================================================================================
// blockchain/bridge/mod.rs 测试
// ================================================================================

#[test]
fn test_bridge_transaction_status_enum() {
    use defi_hot_wallet::blockchain::bridge::BridgeTransactionStatus;
    
    let initiated = BridgeTransactionStatus::Initiated;
    let in_transit = BridgeTransactionStatus::InTransit;
    let completed = BridgeTransactionStatus::Completed;
    let _failed = BridgeTransactionStatus::Failed("timeout".to_string());
    
    // 测试 Clone
    let initiated_clone = initiated.clone();
    assert!(matches!(initiated_clone, BridgeTransactionStatus::Initiated));
    
    // 测试 PartialEq
    assert_eq!(initiated, BridgeTransactionStatus::Initiated);
    assert_eq!(in_transit, BridgeTransactionStatus::InTransit);
    assert_eq!(completed, BridgeTransactionStatus::Completed);
    
    assert_ne!(initiated, completed);
}

#[test]
fn test_bridge_transaction_status_failed() {
    use defi_hot_wallet::blockchain::bridge::BridgeTransactionStatus;
    
    let failed1 = BridgeTransactionStatus::Failed("error1".to_string());
    let failed2 = BridgeTransactionStatus::Failed("error1".to_string());
    let failed3 = BridgeTransactionStatus::Failed("error2".to_string());
    
    assert_eq!(failed1, failed2);
    assert_ne!(failed1, failed3);
}

#[test]
fn test_bridge_transaction_struct() {
    use defi_hot_wallet::blockchain::bridge::{BridgeTransaction, BridgeTransactionStatus};
    use chrono::Utc;
    
    let now = Utc::now();
    let tx = BridgeTransaction {
        id: "tx123".to_string(),
        from_wallet: "wallet1".to_string(),
        from_chain: "ethereum".to_string(),
        to_chain: "polygon".to_string(),
        token: "USDC".to_string(),
        amount: "100.0".to_string(),
        status: BridgeTransactionStatus::Initiated,
        source_tx_hash: Some("0xabc".to_string()),
        destination_tx_hash: None,
        created_at: now,
        updated_at: now,
        fee_amount: Some("0.5".to_string()),
        estimated_completion_time: None,
    };
    
    assert_eq!(tx.id, "tx123");
    assert_eq!(tx.from_chain, "ethereum");
    assert_eq!(tx.to_chain, "polygon");
    assert_eq!(tx.amount, "100.0");
    
    // 测试 Clone
    let tx_clone = tx.clone();
    assert_eq!(tx_clone.id, tx.id);
}

#[test]
fn test_bridge_transaction_debug() {
    use defi_hot_wallet::blockchain::bridge::{BridgeTransaction, BridgeTransactionStatus};
    use chrono::Utc;
    
    let tx = BridgeTransaction {
        id: "test".to_string(),
        from_wallet: "w1".to_string(),
        from_chain: "eth".to_string(),
        to_chain: "sol".to_string(),
        token: "USDC".to_string(),
        amount: "10".to_string(),
        status: BridgeTransactionStatus::Completed,
        source_tx_hash: None,
        destination_tx_hash: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        fee_amount: None,
        estimated_completion_time: None,
    };
    
    let debug_str = format!("{:?}", tx);
    assert!(debug_str.contains("test"));
}

// ================================================================================
// core/wallet/backup.rs 测试
// ================================================================================

#[tokio::test]
async fn test_backup_wallet() {
    use defi_hot_wallet::core::wallet::backup::backup_wallet;
    use defi_hot_wallet::storage::WalletStorage;
    use std::sync::Arc;
    
    let storage = WalletStorage::new().await.unwrap();
    let storage_arc: Arc<dyn defi_hot_wallet::storage::WalletStorageTrait + Send + Sync> = Arc::new(storage);
    
    let result = backup_wallet(&storage_arc, "test_wallet").await;
    assert!(result.is_ok(), "backup_wallet should succeed");
    
    let mnemonic = result.unwrap();
    assert!(mnemonic.len() > 0, "Mnemonic should not be empty");
}

#[tokio::test]
async fn test_backup_wallet_empty_name() {
    use defi_hot_wallet::core::wallet::backup::backup_wallet;
    use defi_hot_wallet::storage::WalletStorage;
    use std::sync::Arc;
    
    let storage = WalletStorage::new().await.unwrap();
    let storage_arc: Arc<dyn defi_hot_wallet::storage::WalletStorageTrait + Send + Sync> = Arc::new(storage);
    
    let result = backup_wallet(&storage_arc, "").await;
    // 即使名称为空，生成助记词也应该成功
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_backup_wallet_special_chars() {
    use defi_hot_wallet::core::wallet::backup::backup_wallet;
    use defi_hot_wallet::storage::WalletStorage;
    use std::sync::Arc;
    
    let storage = WalletStorage::new().await.unwrap();
    let storage_arc: Arc<dyn defi_hot_wallet::storage::WalletStorageTrait + Send + Sync> = Arc::new(storage);
    
    let result = backup_wallet(&storage_arc, "wallet@#$%").await;
    assert!(result.is_ok());
}

// ================================================================================
// tools/serdes.rs 测试 - Group serialization
// ================================================================================

// 注意：由于 serdes.rs 中大部分功能被注释掉（等待 elliptic_curve 升级），
// 我们只能测试可用的 group 序列化功能

// 暂时跳过这些测试，因为需要具体的 Group 类型实现
// 等待项目升级 elliptic_curve 到 0.10+ 版本后再添加完整测试

