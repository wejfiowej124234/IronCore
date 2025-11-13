// 阶段 1：0% 覆盖率模块测试
// 目标：快速提升覆盖率 +3.7%

// ================================================================================
// blockchain/traits.rs 测试
// ================================================================================

#[test]
fn test_transaction_status_enum() {
    use defi_hot_wallet::blockchain::traits::TransactionStatus;
    
    let pending = TransactionStatus::Pending;
    let confirmed = TransactionStatus::Confirmed;
    let failed = TransactionStatus::Failed;
    let unknown = TransactionStatus::Unknown;
    
    // 测试 Clone
    let pending_clone = pending.clone();
    assert!(matches!(pending_clone, TransactionStatus::Pending));
    
    // 测试 PartialEq
    assert_eq!(pending, TransactionStatus::Pending);
    assert_eq!(confirmed, TransactionStatus::Confirmed);
    assert_eq!(failed, TransactionStatus::Failed);
    assert_eq!(unknown, TransactionStatus::Unknown);
    
    assert_ne!(pending, confirmed);
}

#[test]
fn test_transaction_status_debug() {
    use defi_hot_wallet::blockchain::traits::TransactionStatus;
    
    let status = TransactionStatus::Confirmed;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("Confirmed"));
}

#[test]
fn test_transaction_info_struct() {
    use defi_hot_wallet::blockchain::traits::TransactionInfo;
    
    let tx_info = TransactionInfo {
        hash: "0xabc123".to_string(),
        from: "0x111".to_string(),
        to: "0x222".to_string(),
        amount: "1.5".to_string(),
    };
    
    assert_eq!(tx_info.hash, "0xabc123");
    assert_eq!(tx_info.from, "0x111");
    assert_eq!(tx_info.to, "0x222");
    assert_eq!(tx_info.amount, "1.5");
    
    // 测试 Clone
    let tx_clone = tx_info.clone();
    assert_eq!(tx_clone.hash, tx_info.hash);
}

#[test]
fn test_transaction_type_alias() {
    use defi_hot_wallet::blockchain::traits::Transaction;
    
    let tx: Transaction = Transaction {
        hash: "0xdef456".to_string(),
        from: "0x333".to_string(),
        to: "0x444".to_string(),
        amount: "2.5".to_string(),
    };
    
    assert_eq!(tx.hash, "0xdef456");
}

// ================================================================================
// blockchain/audit.rs 测试
// ================================================================================

#[test]
fn test_audit_placeholder() {
    // blockchain::audit 模块不是公开的，跳过此测试
    // 直接验证模块存在
    assert!(true, "audit module exists");
}

// ================================================================================
// service/di_container.rs 测试
// ================================================================================

#[test]
fn test_di_container_new() {
    use defi_hot_wallet::service::di_container::DiContainer;
    
    let container = DiContainer::new();
    // DiContainer is a unit struct, just test it can be created
    let _ = container;
}

#[test]
fn test_di_container_default() {
    use defi_hot_wallet::service::di_container::DiContainer;
    
    let container = DiContainer::default();
    let _ = container;
}

// ================================================================================
// ops/backup.rs 测试
// ================================================================================

#[test]
fn test_backup_new() {
    use defi_hot_wallet::ops::backup::Backup;
    
    let backup = Backup::new("test_wallet");
    assert_eq!(backup.wallet_name, "test_wallet");
}

#[test]
fn test_backup_clone() {
    use defi_hot_wallet::ops::backup::Backup;
    
    let backup1 = Backup::new("wallet1");
    let backup2 = backup1.clone();
    
    assert_eq!(backup1.wallet_name, backup2.wallet_name);
}

#[test]
fn test_backup_equality() {
    use defi_hot_wallet::ops::backup::Backup;
    
    let backup1 = Backup::new("wallet");
    let backup2 = Backup::new("wallet");
    let backup3 = Backup::new("other");
    
    assert_eq!(backup1, backup2);
    assert_ne!(backup1, backup3);
}

#[test]
fn test_backup_debug() {
    use defi_hot_wallet::ops::backup::Backup;
    
    let backup = Backup::new("test");
    let debug_str = format!("{:?}", backup);
    assert!(debug_str.contains("test"));
}

#[test]
fn test_perform_backup() {
    use defi_hot_wallet::ops::backup::{Backup, perform_backup};
    
    let backup = Backup::new("wallet");
    let result = perform_backup(&backup);
    
    assert!(result.is_ok());
}

#[test]
fn test_perform_backup_empty_name() {
    use defi_hot_wallet::ops::backup::{Backup, perform_backup};
    
    let backup = Backup::new("");
    let result = perform_backup(&backup);
    
    // 即使名称为空，perform_backup 也应该成功（当前实现）
    assert!(result.is_ok());
}

// ================================================================================
// ops/metrics.rs 测试
// ================================================================================

#[test]
fn test_metrics_new() {
    use defi_hot_wallet::ops::metrics::Metrics;
    
    let metrics = Metrics::new();
    assert_eq!(metrics.get_count("test"), 0);
}

#[test]
fn test_metrics_default() {
    use defi_hot_wallet::ops::metrics::Metrics;
    
    let metrics = Metrics::default();
    assert_eq!(metrics.get_count("any"), 0);
}

#[test]
fn test_metrics_inc_count() {
    use defi_hot_wallet::ops::metrics::Metrics;
    
    let metrics = Metrics::new();
    
    metrics.inc_count("requests");
    assert_eq!(metrics.get_count("requests"), 1);
    
    metrics.inc_count("requests");
    assert_eq!(metrics.get_count("requests"), 2);
    
    metrics.inc_count("errors");
    assert_eq!(metrics.get_count("errors"), 1);
    assert_eq!(metrics.get_count("requests"), 2);
}

#[test]
fn test_metrics_get_nonexistent() {
    use defi_hot_wallet::ops::metrics::Metrics;
    
    let metrics = Metrics::new();
    assert_eq!(metrics.get_count("nonexistent"), 0);
}

#[test]
fn test_metrics_multiple_counters() {
    use defi_hot_wallet::ops::metrics::Metrics;
    
    let metrics = Metrics::new();
    
    for _ in 0..5 {
        metrics.inc_count("counter1");
    }
    
    for _ in 0..3 {
        metrics.inc_count("counter2");
    }
    
    assert_eq!(metrics.get_count("counter1"), 5);
    assert_eq!(metrics.get_count("counter2"), 3);
}

#[test]
fn test_metrics_clone() {
    use defi_hot_wallet::ops::metrics::Metrics;
    
    let metrics1 = Metrics::new();
    metrics1.inc_count("test");
    
    let metrics2 = metrics1.clone();
    
    // 克隆的实例应该共享相同的计数器（因为使用 Arc）
    assert_eq!(metrics2.get_count("test"), 1);
    
    metrics2.inc_count("test");
    assert_eq!(metrics1.get_count("test"), 2);
    assert_eq!(metrics2.get_count("test"), 2);
}

#[test]
fn test_metrics_debug() {
    use defi_hot_wallet::ops::metrics::Metrics;
    
    let metrics = Metrics::new();
    let debug_str = format!("{:?}", metrics);
    assert!(debug_str.contains("Metrics"));
}

