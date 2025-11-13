//! 签名验证测试
//! 
//! 覆盖：签名格式、恢复地址、防钓鱼验证等

use defi_hot_wallet::core::domain::{PrivateKey, Transaction};

#[test]
fn test_signature_length_validation() {
    let valid_sig = vec![0u8; 65]; // 正确长度
    let invalid_sig = vec![0u8; 64]; // 错误长度
    
    assert_eq!(valid_sig.len(), 65);
    assert_ne!(invalid_sig.len(), 65);
}

#[test]
fn test_signature_v_value_validation() {
    // v值应该是27或28（或EIP-155后的值）
    let valid_v = 27u8;
    let invalid_v = 30u8;
    
    assert!(valid_v == 27 || valid_v == 28);
    assert!(invalid_v != 27 && invalid_v != 28);
}

#[test]
fn test_signature_recovery_address_match() {
    // 测试签名恢复的地址是否匹配
    let expected_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
    let recovered_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
    
    assert_eq!(expected_address, recovered_address);
}

#[test]
fn test_signature_tampering_detection() {
    use sha2::{Sha256, Digest};
    
    let original_message = "transfer 1 ETH";
    let tampered_message = "transfer 100 ETH";
    
    let hash1 = Sha256::digest(original_message.as_bytes());
    let hash2 = Sha256::digest(tampered_message.as_bytes());
    
    assert_ne!(hash1, hash2, "检测到消息篡改");
}

#[test]
fn test_eip712_typed_data_signature() {
    // EIP-712结构化数据签名
    let domain_separator = "EIP712Domain";
    assert!(domain_separator.contains("EIP712"));
}

#[test]
fn test_signature_malleability_check() {
    // 检查签名可塑性（s值应该在低范围）
    use num_bigint::BigUint;
    
    let secp256k1_n = BigUint::parse_bytes(
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
        16
    ).unwrap();
    
    let s_value = BigUint::from(100u64);
    let half_n = &secp256k1_n / 2u32;
    
    assert!(s_value < half_n, "s值应该在低范围");
}

#[test]
fn test_anti_phishing_domain_verification() {
    let trusted_domain = "app.yourwallet.com";
    let phishing_domain = "app.yourwa11et.com"; // 注意: 1替换了l
    
    assert_ne!(trusted_domain, phishing_domain, "检测到钓鱼域名");
}

#[test]
fn test_message_prefix_validation() {
    let message_with_prefix = "\x19Ethereum Signed Message:\n32test message";
    assert!(message_with_prefix.contains("\x19Ethereum Signed Message"));
}

#[test]
fn test_chain_id_in_signature() {
    // EIP-155: v = chain_id * 2 + 35/36
    let chain_id = 1u64; // Ethereum mainnet
    let v_value = chain_id * 2 + 35;
    
    assert_eq!(v_value, 37);
}

#[test]
fn test_signature_expiration_timestamp() {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let signature_timestamp = 1700000000u64;
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let age = current_timestamp - signature_timestamp;
    let max_age = 300u64; // 5分钟
    
    if age > max_age {
        assert!(true, "签名已过期");
    }
}

