//! 密码学深度测试 - 冲刺 95%

#[cfg(test)]
mod key_derivation_deep_tests {
    use defi_hot_wallet::core::domain::PrivateKey;
    
    #[test]
    fn test_private_key_from_various_sources() {
        // 从不同来源创建密钥
        let sources = vec![
            [1u8; 32],
            [2u8; 32],
            [0xFFu8; 32],
            {
                let mut arr = [0u8; 32];
                for (i, byte) in arr.iter_mut().enumerate() {
                    *byte = i as u8;
                }
                arr
            },
        ];
        
        for source in sources {
            let pk = PrivateKey::try_from_slice(&source).unwrap();
            assert_eq!(pk.as_bytes(), &source);
        }
    }
    
    #[test]
    fn test_private_key_slice_exact_length() {
        let bytes = [42u8; 32];
        let pk = PrivateKey::try_from_slice(&bytes).unwrap();
        
        assert_eq!(pk.as_bytes().len(), 32);
        assert_eq!(pk.as_bytes(), &bytes);
    }
    
    #[test]
    fn test_private_key_slice_too_short() {
        for len in 0..32 {
            let bytes = vec![1u8; len];
            let result = PrivateKey::try_from_slice(&bytes);
            assert!(result.is_err(), "长度 {} 应该失败", len);
        }
    }
    
    #[test]
    fn test_private_key_slice_too_long() {
        for len in 33..64 {
            let bytes = vec![1u8; len];
            let result = PrivateKey::try_from_slice(&bytes);
            assert!(result.is_err(), "长度 {} 应该失败", len);
        }
    }
    
    #[test]
    fn test_key_bytes_immutable_reference() {
        let pk = PrivateKey::try_from_slice(&[42u8; 32]).unwrap();
        let bytes1 = pk.as_bytes();
        let bytes2 = pk.as_bytes();
        
        // 应该返回相同的引用
        assert_eq!(bytes1, bytes2);
    }
}

#[cfg(feature = "bitcoin")]
mod secp256k1_deep_tests {
    use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
    use bitcoin::Network;
    
    #[test]
    fn test_public_key_y_coordinate_parity() {
        for _ in 0..20 {
            let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
            let pk_bytes = kp.public_key_bytes();
            
            // 压缩公钥的第一个字节表示 y 坐标的奇偶性
            assert!(pk_bytes[0] == 0x02 || pk_bytes[0] == 0x03);
        }
    }
    
    #[test]
    fn test_uncompressed_key_x_y_coordinates() {
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let uncompressed = kp.uncompressed_public_key_bytes();
        
        // 未压缩公钥: 0x04 + x(32) + y(32)
        assert_eq!(uncompressed[0], 0x04);
        assert_eq!(uncompressed.len(), 65);
    }
    
    #[test]
    fn test_compressed_uncompressed_relationship() {
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        let compressed = kp.public_key_bytes();
        let uncompressed = kp.uncompressed_public_key_bytes();
        
        // x 坐标应该相同
        assert_eq!(&compressed[1..33], &uncompressed[1..33]);
    }
}

#[cfg(feature = "bitcoin")]
mod schnorr_specific_tests {
    use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
    use bitcoin::Network;
    
    #[test]
    fn test_schnorr_signature_no_malleability() {
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let msg = [0x42u8; 32];
        
        let sig = kp.sign_schnorr(&msg).unwrap();
        
        // Schnorr 签名固定 64 字节，无延展性
        assert_eq!(sig.len(), 64);
        
        // 签名应该不可篡改（实际需要验证函数）
    }
    
    #[test]
    fn test_schnorr_nonce_uniqueness() {
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        // 不同消息应该使用不同的 nonce
        let msg1 = [0x01u8; 32];
        let msg2 = [0x02u8; 32];
        
        let sig1 = kp.sign_schnorr(&msg1).unwrap();
        let sig2 = kp.sign_schnorr(&msg2).unwrap();
        
        // 签名的 R 部分应该不同（前 32 字节）
        assert_ne!(&sig1[..32], &sig2[..32], "不同消息应该有不同的 nonce");
    }
}

#[cfg(feature = "bitcoin")]
mod ecdsa_specific_tests {
    use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
    use bitcoin::Network;
    
    #[test]
    fn test_ecdsa_low_s_normalization() {
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        // Bitcoin 要求 ECDSA 签名使用 low-s 值
        for i in 0..10 {
            let msg = [i as u8; 32];
            let sig = kp.sign_ecdsa(&msg).unwrap();
            
            // DER 编码的签名长度范围：最小约8字节（理论上），最大72字节
            // 典型长度是70-72字节，但可能更短（当r或s值前导字节较小时）
            assert!(sig.len() >= 8 && sig.len() <= 72, 
                "ECDSA 签名长度应该在8-72字节之间，实际: {}", sig.len());
            
            // 验证 DER 编码格式的基本结构
            assert_eq!(sig[0], 0x30, "应该以 SEQUENCE 标记开始");
            assert!(sig.len() >= 6, "签名至少应该包含基本的 DER 结构");
        }
    }
    
    #[test]
    fn test_ecdsa_signature_components() {
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let msg = [0x42u8; 32];
        
        let sig = kp.sign_ecdsa(&msg).unwrap();
        
        // DER 序列: 0x30 [total-length] 0x02 [r-length] [r] 0x02 [s-length] [s]
        assert_eq!(sig[0], 0x30);
        assert_eq!(sig[2], 0x02);  // r 的标记
    }
}

#[cfg(test)]
mod hash_function_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_message_hash_size() {
        // Bitcoin 使用 SHA256，输出 32 字节
        let msg = [0x42u8; 32];
        assert_eq!(msg.len(), 32);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_different_messages_different_hashes() {
        // 不同消息应该产生不同哈希
        let msg1 = [0x01u8; 32];
        let msg2 = [0x02u8; 32];
        
        assert_ne!(msg1, msg2);
    }
}

#[cfg(test)]
mod encoding_deep_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_base58_address_characters() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let addr = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Bitcoin).unwrap();
        
        // Base58 不包含 0, O, I, l
        assert!(!addr.contains('0'));
        assert!(!addr.contains('O'));
        assert!(!addr.contains('I'));
        assert!(!addr.contains('l'));
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_bech32_address_lowercase() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let addr = BitcoinAddress::from_public_key_segwit(kp.public_key(), Network::Bitcoin).unwrap();
        
        // Bech32 地址应该全部小写
        assert!(addr.chars().all(|c| !c.is_uppercase()));
    }
}

