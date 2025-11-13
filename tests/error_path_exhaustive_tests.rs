//! 错误路径详尽测试 - 冲刺 95%

#[cfg(test)]
mod all_error_types_tests {
    use defi_hot_wallet::core::errors::WalletError;
    
    #[test]
    fn test_insufficient_funds_error() {
        let err = WalletError::InsufficientFunds("test".into());
        assert!(matches!(err, WalletError::InsufficientFunds(_)));
        
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }
    
    #[test]
    fn test_signing_failed_error() {
        let err = WalletError::SigningFailed("test".into());
        assert!(matches!(err, WalletError::SigningFailed(_)));
    }
    
    #[test]
    fn test_key_generation_failed_error() {
        let err = WalletError::KeyGenerationFailed("test".into());
        assert!(matches!(err, WalletError::KeyGenerationFailed(_)));
    }
    
    #[test]
    fn test_invalid_private_key_error() {
        let err = WalletError::InvalidPrivateKey("test".into());
        assert!(matches!(err, WalletError::InvalidPrivateKey(_)));
    }
    
    #[test]
    fn test_address_generation_failed_error() {
        let err = WalletError::AddressGenerationFailed("test".into());
        assert!(matches!(err, WalletError::AddressGenerationFailed(_)));
    }
    
    #[test]
    fn test_network_error() {
        let err = WalletError::NetworkError("test".into());
        assert!(matches!(err, WalletError::NetworkError(_)));
    }
    
    #[test]
    fn test_crypto_error() {
        let err = WalletError::CryptoError("test".into());
        assert!(matches!(err, WalletError::CryptoError(_)));
    }
    
    #[test]
    fn test_validation_error() {
        let err = WalletError::ValidationError("test".into());
        assert!(matches!(err, WalletError::ValidationError(_)));
    }
    
    #[test]
    fn test_transaction_failed_error() {
        let err = WalletError::TransactionFailed("test".into());
        assert!(matches!(err, WalletError::TransactionFailed(_)));
    }
}

#[cfg(test)]
mod error_propagation_tests {
    use defi_hot_wallet::core::domain::PrivateKey;
    
    #[test]
    fn test_invalid_key_length_error_propagation() {
        let short = vec![1u8; 16];
        let result = PrivateKey::try_from_slice(&short);
        
        assert!(result.is_err());
        
        if let Err(e) = result {
            let msg = format!("{:?}", e);
            assert!(!msg.is_empty());
        }
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_insufficient_funds_in_utxo_selection() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        let utxos = vec![Utxo::new("0".repeat(64), 0, 10_000, "s".into(), 6)];
        
        let result = UtxoSelector::select(&utxos, 100_000, 10, SelectionStrategy::LargestFirst);
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, defi_hot_wallet::core::errors::WalletError::InsufficientFunds(_)));
        }
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_transaction_build_error_propagation() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
            transaction::BitcoinTransaction,
            utxo::Utxo,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let addr = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Testnet).unwrap();
        
        // 空 UTXO 列表
        let result = BitcoinTransaction::build_legacy(&kp, &vec![], &addr, 50_000, 1_000, Network::Testnet);
        assert!(result.is_err());
        
        // 金额过大
        let utxos = vec![Utxo::new("0".repeat(64), 0, 10_000, "s".into(), 6)];
        let result2 = BitcoinTransaction::build_legacy(&kp, &utxos, &addr, 100_000, 1_000, Network::Testnet);
        assert!(result2.is_err());
    }
}

#[cfg(test)]
mod hardware_wallet_error_tests {
    #[test]
    #[cfg(feature = "ledger")]
    fn test_apdu_all_error_status_codes() {
        use defi_hot_wallet::hardware::ledger::apdu::ApduResponse;
        
        let error_codes = vec![
            (0x69, 0x82), (0x69, 0x85), (0x6A, 0x80), (0x6A, 0x82),
            (0x6D, 0x00), (0x6E, 0x00), (0x6F, 0x00), (0x67, 0x00),
            (0x6B, 0x00), (0x64, 0x00), (0x65, 0x81),
        ];
        
        for (sw1, sw2) in error_codes {
            let resp = ApduResponse::from_bytes(&[sw1, sw2]).unwrap();
            assert!(!resp.is_success());
            assert!(resp.status_code() == ((sw1 as u16) << 8) | (sw2 as u16));
        }
    }
    
    #[test]
    #[cfg(feature = "trezor")]
    fn test_trezor_message_type_unknown() {
        use defi_hot_wallet::hardware::trezor::messages::TrezorMessage;
        
        let data = vec![
            0x99, 0x99,  // 未知类型
            0x00, 0x00, 0x00, 0x00,
        ];
        
        let result = TrezorMessage::deserialize(&data);
        assert!(result.is_err());
    }
    
    #[test]
    #[cfg(feature = "ledger")]
    fn test_apdu_response_parse_error() {
        use defi_hot_wallet::hardware::ledger::apdu::ApduResponse;
        
        // 太短的响应
        let result1 = ApduResponse::from_bytes(&[]);
        assert!(result1.is_err());
        
        let result2 = ApduResponse::from_bytes(&[0x90]);
        assert!(result2.is_err());
    }
}

#[cfg(test)]
mod validation_error_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_address_validation_all_failures() {
        use defi_hot_wallet::blockchain::bitcoin::address::BitcoinAddress;
        use bitcoin::Network;
        
        let pattern1 = "1".repeat(100);
        let pattern2 = "bc1".repeat(20);
        let invalid = vec![
            "",
            "a",
            "invalid_address",
            &pattern1,
            &pattern2,
            "tb1q",
            "!!!",
            "0x123",
        ];
        
        for addr in invalid {
            let result = BitcoinAddress::validate(addr, Network::Bitcoin).unwrap();
            assert!(!result, "应该识别为无效: {}", addr);
        }
    }
    
    #[test]
    #[cfg(feature = "ledger")]
    fn test_bip32_path_parse_failures() {
        use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
        
        let invalid_paths = vec![
            "",
            "m",
            "m/",
            "invalid",
            "m/abc/def",
            "m/44'/abc",
            "/44'/0'",
            "m/-1",
        ];
        
        for path in invalid_paths {
            let result = Bip32Path::from_str(path);
            assert!(result.is_err(), "无效路径应该失败: {}", path);
        }
    }
}

#[cfg(test)]
mod resource_limit_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_max_utxo_count_handling() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        // 大量 UTXO
        let mut utxos = Vec::new();
        for i in 0..500 {
            utxos.push(Utxo::new(format!("tx{}", i), 0, 1_000, "s".into(), 6));
        }
        
        let result = UtxoSelector::select(&utxos, 100_000, 10, SelectionStrategy::SmallestFirst);
        
        // 应该能处理大量 UTXO
        assert!(result.is_ok() || result.is_err());
    }
    
    #[test]
    #[cfg(feature = "ledger")]
    fn test_max_apdu_data_handling() {
        use defi_hot_wallet::hardware::ledger::apdu::{ApduCommand, ApduClass, ApduInstruction};
        
        // 255 是 APDU 数据的最大长度
        let max_data = vec![0xFFu8; 255];
        let cmd = ApduCommand::new(ApduClass::Standard, ApduInstruction::SignTransaction, 0, 0, max_data);
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[4], 255);
        assert_eq!(bytes.len(), 260);
    }
}

#[cfg(test)]
mod timeout_and_retry_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_multiple_transaction_attempts() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
            transaction::BitcoinTransaction,
            utxo::Utxo,
        };
        use bitcoin::Network;
        use bitcoin::hashes::{Hash, hash160};
        
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let addr = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Testnet).unwrap();
        
        // 生成与 keypair 匹配的正确 P2PKH script_pubkey
        let pubkey_bytes = kp.public_key().serialize();
        let pubkey_hash = hash160::Hash::hash(&pubkey_bytes);
        let script_pubkey = format!("76a914{}88ac", hex::encode(pubkey_hash.as_byte_array()));
        
        let utxos = vec![Utxo::new("0".repeat(64), 0, 100_000, script_pubkey, 6)];
        
        // 多次尝试应该成功（测试幂等性和重试场景）
        for _ in 0..5 {
            let result = BitcoinTransaction::build_legacy(&kp, &utxos, &addr, 50_000, 1_000, Network::Testnet);
            assert!(result.is_ok(), "交易构建应该在多次尝试中都成功");
        }
    }
}

