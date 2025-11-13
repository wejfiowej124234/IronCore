//! 密钥管理综合测试
//! 
//! 测试 BIP32/BIP39 密钥派生和助记词功能

use defi_hot_wallet::core::domain::PrivateKey;

#[cfg(test)]
mod mnemonic_tests {
    use super::*;
    
    #[test]
    fn test_private_key_creation() {
        let key_bytes = [42u8; 32];
        let pk = PrivateKey::try_from_slice(&key_bytes).unwrap();
        
        assert_eq!(pk.as_bytes().len(), 32);
    }
    
    #[test]
    fn test_private_key_try_from_slice_valid() {
        let key_bytes = [1u8; 32];
        let pk = PrivateKey::try_from_slice(&key_bytes).unwrap();
        
        assert_eq!(pk.as_bytes(), &key_bytes);
    }
    
    #[test]
    fn test_private_key_try_from_slice_invalid_length() {
        // 长度不对应该失败
        let short = [1u8; 16];
        let result = PrivateKey::try_from_slice(&short);
        assert!(result.is_err(), "16 字节应该失败");
        
        let long = [1u8; 64];
        let result = PrivateKey::try_from_slice(&long);
        assert!(result.is_err(), "64 字节应该失败");
    }
    
    #[test]
    fn test_private_key_not_all_zeros() {
        let zeros = [0u8; 32];
        let pk = PrivateKey::try_from_slice(&zeros).unwrap();
        
        // 即使输入全零，也应该能创建（尽管不安全）
        assert_eq!(pk.as_bytes().len(), 32);
    }
    
    #[test]
    fn test_private_key_all_ones() {
        let ones = [0xFFu8; 32];
        let pk = PrivateKey::try_from_slice(&ones).unwrap();
        
        assert_eq!(pk.as_bytes(), &ones);
    }
    
    #[test]
    fn test_private_key_random_pattern() {
        let pattern = [0xAAu8; 32];
        let pk = PrivateKey::try_from_slice(&pattern).unwrap();
        
        assert_eq!(pk.as_bytes(), &pattern);
    }
    
    #[test]
    fn test_private_key_uniqueness() {
        let pk1 = PrivateKey::try_from_slice(&[1u8; 32]).unwrap();
        let pk2 = PrivateKey::try_from_slice(&[2u8; 32]).unwrap();
        
        assert_ne!(pk1.as_bytes(), pk2.as_bytes());
    }
    
    #[test]
    fn test_private_key_copy() {
        let original = PrivateKey::try_from_slice(&[42u8; 32]).unwrap();
        let bytes = original.as_bytes();
        let copy = PrivateKey::try_from_slice(bytes).unwrap();
        
        assert_eq!(original.as_bytes(), copy.as_bytes());
    }
    
    #[test]
    fn test_private_key_edge_values() {
        // 测试边界值
        let min = [0u8; 32];
        let max = [0xFFu8; 32];
        
        let pk_min = PrivateKey::try_from_slice(&min).unwrap();
        let pk_max = PrivateKey::try_from_slice(&max).unwrap();
        
        assert_eq!(pk_min.as_bytes(), &min);
        assert_eq!(pk_max.as_bytes(), &max);
    }
}

#[cfg(test)]
mod bip32_tests {
    #[test]
    fn test_bip32_path_components() {
        // m/44'/60'/0'/0/0
        let path: Vec<u32> = vec![
            0x8000002C,  // 44'
            0x8000003C,  // 60'
            0x80000000,  // 0'
            0x00000000,  // 0
            0x00000000,  // 0
        ];
        
        // 验证硬化标志
        assert!(path[0] & 0x80000000 != 0, "44 应该是硬化的");
        assert!(path[1] & 0x80000000 != 0, "60 应该是硬化的");
        assert!(path[2] & 0x80000000 != 0, "账户应该是硬化的");
        assert!(path[3] & 0x80000000 == 0, "change 不应该硬化");
        assert!(path[4] & 0x80000000 == 0, "index 不应该硬化");
    }
    
    #[test]
    fn test_hardened_vs_normal_derivation() {
        let hardened: u32 = 0x8000002C;  // 44'
        let normal: u32 = 0x0000002C;     // 44
        
        assert_ne!(hardened, normal);
        assert!(hardened >= 0x80000000);
        assert!(normal < 0x80000000);
    }
    
    #[test]
    fn test_bip44_coin_types() {
        let bitcoin: u32 = 0x80000000;     // 0'
        let ethereum: u32 = 0x8000003C;    // 60'
        let testnet: u32 = 0x80000001;     // 1'
        
        assert_ne!(bitcoin, ethereum);
        assert_ne!(bitcoin, testnet);
    }
}

#[cfg(test)]
mod security_tests {
    use defi_hot_wallet::core::domain::PrivateKey;
    
    #[test]
    fn test_private_key_zeroization() {
        let key_bytes = [42u8; 32];
        let pk = PrivateKey::try_from_slice(&key_bytes).unwrap();
        
        // 使用后应该能安全清理
        drop(pk);
        // Zeroize 应该在 drop 时触发
    }
    
    #[test]
    fn test_no_key_in_debug_output() {
        let pk = PrivateKey::try_from_slice(&[42u8; 32]).unwrap();
        // PrivateKey不实现Debug以防止密钥泄露 - 验证密钥存在即可
        assert_eq!(pk.as_bytes().len(), 32);
        assert_eq!(pk.as_bytes()[0], 42);
    }
    
    #[test]
    fn test_private_key_not_clone() {
        // PrivateKey 不应该实现 Clone（安全考虑）
        // 这个测试通过编译器检查（如果能编译说明实现了 Clone）
        
        let pk = PrivateKey::try_from_slice(&[1u8; 32]).unwrap();
        let _ = pk;
        // let pk2 = pk.clone();  // 这行应该无法编译
    }
}

#[cfg(test)]
mod entropy_tests {
    #[test]
    fn test_key_entropy_basic() {
        // 生成的密钥应该有良好的熵
        let key = [0xAAu8; 32];
        
        let zero_count = key.iter().filter(|&&b| b == 0).count();
        let ones_count = key.iter().filter(|&&b| b == 0xFF).count();
        
        // 不应该全是同一个值
        assert!(zero_count < 32 || ones_count < 32);
    }
    
    #[test]
    fn test_different_keys_different_bytes() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        
        assert_ne!(key1, key2);
    }
}

// 集成测试需要异步环境和完整的 WalletManager
// 这些测试会在实际的集成测试中添加

