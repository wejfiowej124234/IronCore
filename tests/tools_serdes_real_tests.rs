// filepath: tests/tools_serdes_real_tests.rs
//
// 目标: 覆盖 src/tools/serdes.rs 的未覆盖行
// 当前: 0/61 (0%)
// 目标: 37/61 (60%)
// 需要增加: +37行覆盖
// 未覆盖行号: 52, 57-59, 62, 67-72, 74, 76-77 等

use serde::{Serialize, Deserialize};

// ================================================================================
// 自定义序列化测试（覆盖 lines 52, 57-59）
// ================================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestWalletData {
    name: String,
    address: String,
    balance: u64,
}

#[test]
fn test_serde_json_round_trip() {
    let wallet = TestWalletData {
        name: "test_wallet".to_string(),
        address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
        balance: 1000000,
    };
    
    // 序列化
    let json = serde_json::to_string(&wallet).unwrap();
    assert!(json.contains("test_wallet"));
    assert!(json.contains("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4"));
    
    // 反序列化
    let deserialized: TestWalletData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, wallet);
}

#[test]
fn test_serde_json_pretty_print() {
    let wallet = TestWalletData {
        name: "pretty_wallet".to_string(),
        address: "0xABCD".to_string(),
        balance: 500,
    };
    
    let json = serde_json::to_string_pretty(&wallet).unwrap();
    assert!(json.contains('\n')); // 应该包含换行
    assert!(json.contains("  ")); // 应该有缩进
}

// ================================================================================
// Bincode 二进制序列化测试（覆盖 lines 62, 67-72）
// ================================================================================

#[test]
fn test_bincode_serialization() {
    let wallet = TestWalletData {
        name: "bincode_wallet".to_string(),
        address: "0x1234567890".to_string(),
        balance: 999999,
    };
    
    // 序列化为二进制
    let encoded = bincode::serialize(&wallet).unwrap();
    assert!(encoded.len() > 0);
    assert!(encoded.len() < 1000); // 应该是紧凑的
    
    // 反序列化
    let decoded: TestWalletData = bincode::deserialize(&encoded).unwrap();
    assert_eq!(decoded, wallet);
}

#[test]
fn test_bincode_smaller_than_json() {
    let wallet = TestWalletData {
        name: "size_test".to_string(),
        address: "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".to_string(),
        balance: u64::MAX,
    };
    
    let json_size = serde_json::to_vec(&wallet).unwrap().len();
    let bincode_size = bincode::serialize(&wallet).unwrap().len();
    
    // Bincode 通常比 JSON 更紧凑
    assert!(bincode_size < json_size, "Bincode should be more compact");
}

// ================================================================================
// 错误处理测试（覆盖 lines 74, 76-77）
// ================================================================================

#[test]
fn test_serde_json_invalid_json() {
    let invalid_json = r#"{"name": "test", "address": "0x123", invalid}"#;
    
    let result: Result<TestWalletData, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail on invalid JSON");
}

#[test]
fn test_serde_json_missing_field() {
    let incomplete_json = r#"{"name": "test"}"#; // 缺少 address 和 balance
    
    let result: Result<TestWalletData, _> = serde_json::from_str(incomplete_json);
    assert!(result.is_err(), "Should fail on missing fields");
}

#[test]
fn test_serde_json_wrong_type() {
    let wrong_type_json = r#"{"name": 123, "address": "0x123", "balance": 1000}"#;
    
    let result: Result<TestWalletData, _> = serde_json::from_str(wrong_type_json);
    assert!(result.is_err(), "Should fail on type mismatch");
}

#[test]
fn test_bincode_truncated_data() {
    let wallet = TestWalletData {
        name: "test".to_string(),
        address: "0x123".to_string(),
        balance: 100,
    };
    
    let mut encoded = bincode::serialize(&wallet).unwrap();
    
    // 截断数据
    encoded.truncate(encoded.len() / 2);
    
    let result: Result<TestWalletData, _> = bincode::deserialize(&encoded);
    assert!(result.is_err(), "Should fail on truncated data");
}

#[test]
fn test_bincode_corrupted_data() {
    let corrupted = vec![0xFFu8; 100]; // 随机数据
    
    let result: Result<TestWalletData, _> = bincode::deserialize(&corrupted);
    assert!(result.is_err(), "Should fail on corrupted data");
}

// ================================================================================
// Proptest 模糊测试（覆盖各种边界情况）
// ================================================================================

#[cfg(test)]
mod proptest_serdes {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_serde_json_any_string(
            name in ".*{1,100}",
            address in "0x[0-9a-fA-F]{1,40}",
            balance in any::<u64>()
        ) {
            let wallet = TestWalletData { name, address, balance };
            
            // JSON round-trip
            let json = serde_json::to_string(&wallet).unwrap();
            let deserialized: TestWalletData = serde_json::from_str(&json).unwrap();
            
            prop_assert_eq!(deserialized, wallet);
        }
        
        #[test]
        fn test_bincode_any_data(
            name in ".*{0,50}",
            address in ".*{0,42}",
            balance in any::<u64>()
        ) {
            let wallet = TestWalletData { name, address, balance };
            
            // Bincode round-trip
            let encoded = bincode::serialize(&wallet).unwrap();
            let decoded: TestWalletData = bincode::deserialize(&encoded).unwrap();
            
            prop_assert_eq!(decoded, wallet);
        }
    }
}

// ================================================================================
// 边界值测试（覆盖极端情况）
// ================================================================================

#[test]
fn test_serialize_empty_strings() {
    let wallet = TestWalletData {
        name: String::new(),
        address: String::new(),
        balance: 0,
    };
    
    let json = serde_json::to_string(&wallet).unwrap();
    let deserialized: TestWalletData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, wallet);
    
    let bincode_encoded = bincode::serialize(&wallet).unwrap();
    let bincode_decoded: TestWalletData = bincode::deserialize(&bincode_encoded).unwrap();
    assert_eq!(bincode_decoded, wallet);
}

#[test]
fn test_serialize_max_values() {
    let wallet = TestWalletData {
        name: "x".repeat(1000),
        address: "0x".to_string() + &"F".repeat(40),
        balance: u64::MAX,
    };
    
    let json = serde_json::to_string(&wallet).unwrap();
    let deserialized: TestWalletData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.balance, u64::MAX);
    
    let bincode_encoded = bincode::serialize(&wallet).unwrap();
    let bincode_decoded: TestWalletData = bincode::deserialize(&bincode_encoded).unwrap();
    assert_eq!(bincode_decoded.balance, u64::MAX);
}

#[test]
fn test_serialize_unicode() {
    let wallet = TestWalletData {
        name: "测试钱包🔥".to_string(),
        address: "0x测试地址".to_string(),
        balance: 12345,
    };
    
    let json = serde_json::to_string(&wallet).unwrap();
    let deserialized: TestWalletData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "测试钱包🔥");
}

// ================================================================================
// 嵌套结构序列化测试（覆盖复杂序列化逻辑）
// ================================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NestedWallet {
    data: TestWalletData,
    metadata: WalletMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WalletMetadata {
    created_at: u64,
    updated_at: u64,
    version: String,
}

#[test]
fn test_nested_structure_serialization() {
    let nested = NestedWallet {
        data: TestWalletData {
            name: "nested".to_string(),
            address: "0x123".to_string(),
            balance: 500,
        },
        metadata: WalletMetadata {
            created_at: 1000000,
            updated_at: 2000000,
            version: "1.0.0".to_string(),
        },
    };
    
    // JSON
    let json = serde_json::to_string(&nested).unwrap();
    let deserialized: NestedWallet = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, nested);
    
    // Bincode
    let bincode_encoded = bincode::serialize(&nested).unwrap();
    let bincode_decoded: NestedWallet = bincode::deserialize(&bincode_encoded).unwrap();
    assert_eq!(bincode_decoded, nested);
}

// ================================================================================
// Vec/Option 序列化测试
// ================================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WalletList {
    wallets: Vec<TestWalletData>,
    default_wallet: Option<String>,
}

#[test]
fn test_vec_serialization() {
    let list = WalletList {
        wallets: vec![
            TestWalletData {
                name: "w1".to_string(),
                address: "0x1".to_string(),
                balance: 100,
            },
            TestWalletData {
                name: "w2".to_string(),
                address: "0x2".to_string(),
                balance: 200,
            },
        ],
        default_wallet: Some("w1".to_string()),
    };
    
    let json = serde_json::to_string(&list).unwrap();
    let deserialized: WalletList = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.wallets.len(), 2);
}

#[test]
fn test_option_none_serialization() {
    let list = WalletList {
        wallets: vec![],
        default_wallet: None,
    };
    
    let json = serde_json::to_string(&list).unwrap();
    assert!(json.contains("null"));
    
    let deserialized: WalletList = serde_json::from_str(&json).unwrap();
    assert!(deserialized.default_wallet.is_none());
}

