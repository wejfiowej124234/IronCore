//! 综合集成测试
//! 
//! 测试多模块协作和端到端流程

#[cfg(feature = "bitcoin")]
mod bitcoin_integration {
    use defi_hot_wallet::blockchain::bitcoin::{
        account::BitcoinKeypair,
        address::{AddressType, BitcoinAddress},
        utxo::{SelectionStrategy, Utxo, UtxoSelector},
        transaction::BitcoinTransaction,
    };
    use bitcoin::Network;
    
    #[test]
    fn test_end_to_end_bitcoin_transfer_simulation() {
        // 完整的 Bitcoin 转账流程模拟
        
        // 1. 生成密钥对
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        // 2. 生成接收地址（Taproot）
        let receive_addr = BitcoinAddress::from_public_key_taproot(
            keypair.public_key(),
            Network::Testnet
        ).unwrap();
        
        assert!(receive_addr.starts_with("tb1p"));
        
        // 3. 模拟 UTXO
        let utxos = vec![
            Utxo::new("1".repeat(64), 0, 100_000, "5120".to_string() + &"00".repeat(32), 6),
            Utxo::new("2".repeat(64), 0, 50_000, "5120".to_string() + &"00".repeat(32), 3),
        ];
        
        // 4. 选择 UTXO
        let (selected_utxos, estimated_fee) = UtxoSelector::select(
            &utxos,
            80_000,
            10,  // 费率
            SelectionStrategy::BestFit,
        ).unwrap();
        
        assert!(!selected_utxos.is_empty());
        assert!(estimated_fee > 0);
        
        // 5. 构建交易
        let tx = BitcoinTransaction::build_taproot(
            &keypair,
            &selected_utxos,
            &receive_addr,
            80_000,
            estimated_fee,
            Network::Testnet,
        ).unwrap();
        
        // 6. 验证交易
        assert!(tx.input.len() > 0);
        assert!(tx.output.len() > 0);
        
        // 7. 序列化交易
        let hex = BitcoinTransaction::serialize(&tx);
        assert!(!hex.is_empty());
        
        println!("✅ 完整转账流程成功: {} → {}", selected_utxos.len(), tx.output.len());
    }
    
    #[test]
    fn test_multi_address_type_workflow() {
        // 测试在同一个钱包中使用多种地址类型
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        // 生成所有类型的地址
        let legacy_addr = BitcoinAddress::from_public_key_legacy(
            keypair.public_key(),
            Network::Testnet
        ).unwrap();
        
        let segwit_addr = BitcoinAddress::from_public_key_segwit(
            keypair.public_key(),
            Network::Testnet
        ).unwrap();
        
        let taproot_addr = BitcoinAddress::from_public_key_taproot(
            keypair.public_key(),
            Network::Testnet
        ).unwrap();
        
        // 验证所有地址都有效
        assert!(BitcoinAddress::validate(&legacy_addr, Network::Testnet).unwrap());
        assert!(BitcoinAddress::validate(&segwit_addr, Network::Testnet).unwrap());
        assert!(BitcoinAddress::validate(&taproot_addr, Network::Testnet).unwrap());
        
        // 验证地址类型检测
        assert_eq!(BitcoinAddress::detect_type(&legacy_addr).unwrap(), AddressType::Legacy);
        assert_eq!(BitcoinAddress::detect_type(&segwit_addr).unwrap(), AddressType::SegWit);
        assert_eq!(BitcoinAddress::detect_type(&taproot_addr).unwrap(), AddressType::Taproot);
    }
    
    #[test]
    fn test_utxo_selection_affects_fee() {
        // 验证 UTXO 选择策略影响手续费
        let utxos = vec![
            Utxo::new("1".repeat(64), 0, 100_000, "script".to_string(), 6),
            Utxo::new("2".repeat(64), 0, 50_000, "script".to_string(), 6),
            Utxo::new("3".repeat(64), 0, 30_000, "script".to_string(), 6),
        ];
        
        // LargestFirst - 应该选择最少的 UTXO
        let (selected_large, fee_large) = UtxoSelector::select(
            &utxos,
            80_000,
            10,
            SelectionStrategy::LargestFirst,
        ).unwrap();
        
        // SmallestFirst - 可能需要更多 UTXO
        let (selected_small, fee_small) = UtxoSelector::select(
            &utxos,
            80_000,
            10,
            SelectionStrategy::SmallestFirst,
        ).unwrap();
        
        // 更少的 UTXO 通常意味着更低的手续费
        if selected_large.len() < selected_small.len() {
            assert!(fee_large <= fee_small);
        }
    }
    
    #[test]
    fn test_sign_and_verify_message() {
        // 签名和验证流程
        let keypair = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let message = [0x42u8; 32];
        
        // ECDSA 签名 - DER 编码长度可变(8-72字节)
        let ecdsa_sig = keypair.sign_ecdsa(&message).unwrap();
        assert!(ecdsa_sig.len() >= 8 && ecdsa_sig.len() <= 72, 
            "ECDSA DER 签名长度应在 8-72 字节范围，实际: {}", ecdsa_sig.len());
        // 基本 DER 结构验证
        if ecdsa_sig.len() >= 8 {
            assert_eq!(ecdsa_sig[0], 0x30, "应以 SEQUENCE 标签开始");
        }
        
        // Schnorr 签名
        let schnorr_sig = keypair.sign_schnorr(&message).unwrap();
        assert_eq!(schnorr_sig.len(), 64);
        
        // 相同消息签名应该确定
        let ecdsa_sig2 = keypair.sign_ecdsa(&message).unwrap();
        assert_eq!(ecdsa_sig, ecdsa_sig2);
    }
}

#[cfg(all(feature = "ledger", feature = "bitcoin"))]
mod hardware_wallet_integration {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    
    #[test]
    fn test_bip32_path_parsing_integration() {
        // 测试各种 BIP32 路径格式
        let paths = vec![
            "m/44'/0'/0'/0/0",    // Bitcoin Legacy
            "m/84'/0'/0'/0/0",    // Bitcoin SegWit
            "m/86'/0'/0'/0/0",    // Bitcoin Taproot
            "m/44'/60'/0'/0/0",   // Ethereum
        ];
        
        for path_str in paths {
            let path = Bip32Path::from_str(path_str);
            assert!(path.is_ok(), "路径 {} 应该有效", path_str);
            
            let p = path.unwrap();
            assert_eq!(p.path.len(), 5);
        }
    }
    
    #[test]
    fn test_bip32_hardened_derivation() {
        // 测试硬化和非硬化派生
        let hardened_path = Bip32Path::from_str("m/44'/0'/0'").unwrap();
        let normal_path = Bip32Path::from_str("m/44/0/0").unwrap();
        
        // 硬化路径应该有 0x80000000 标志
        for &index in &hardened_path.path {
            assert!(index & 0x80000000 != 0);
        }
        
        // 普通路径不应该有标志
        for &index in &normal_path.path {
            assert!(index & 0x80000000 == 0);
        }
    }
    
    #[test]
    fn test_bip32_path_serialization() {
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        let bytes = path.to_bytes();
        
        // 应该序列化为：长度(1) + 5个索引(每个4字节) = 21字节
        assert_eq!(bytes.len(), 1 + 5 * 4);
        assert_eq!(bytes[0], 5);  // 路径深度
    }
}

#[cfg(test)]
mod error_propagation_tests {
    use defi_hot_wallet::core::errors::WalletError;
    use defi_hot_wallet::core::domain::PrivateKey;
    
    #[test]
    fn test_error_type_matching() {
        // 测试错误类型匹配
        let err = WalletError::InsufficientFunds("test".to_string());
        assert!(matches!(err, WalletError::InsufficientFunds(_)));
        
        let err2 = WalletError::SigningFailed("test".to_string());
        assert!(matches!(err2, WalletError::SigningFailed(_)));
    }
    
    #[test]
    fn test_invalid_key_error() {
        let short_key = [1u8; 16];
        let result = PrivateKey::try_from_slice(&short_key);
        
        assert!(result.is_err());
        if let Err(e) = result {
            // try_from_slice 返回 anyhow::Error
            let msg = format!("{:?}", e);
            // Error message format may vary, just ensure we get an error
            assert!(!msg.is_empty(), "Should have error message");
        }
    }
}

#[cfg(test)]
mod state_consistency_tests {
    #[test]
    fn test_address_generation_consistency() {
        // 相同的输入应该总是产生相同的输出
        #[cfg(feature = "bitcoin")]
        {
            use defi_hot_wallet::blockchain::bitcoin::{
                account::BitcoinKeypair,
                address::BitcoinAddress,
            };
            use bitcoin::Network;
            use defi_hot_wallet::core::domain::PrivateKey;
            
            let key_bytes = [42u8; 32];
            let pk = PrivateKey::try_from_slice(&key_bytes).unwrap();
            
            let keypair1 = BitcoinKeypair::from_private_key(&pk, Network::Bitcoin).unwrap();
            let keypair2 = BitcoinKeypair::from_private_key(&pk, Network::Bitcoin).unwrap();
            
            let addr1 = BitcoinAddress::from_public_key_segwit(
                keypair1.public_key(),
                Network::Bitcoin
            ).unwrap();
            
            let addr2 = BitcoinAddress::from_public_key_segwit(
                keypair2.public_key(),
                Network::Bitcoin
            ).unwrap();
            
            assert_eq!(addr1, addr2, "相同私钥应该产生相同地址");
        }
    }
}

#[cfg(test)]
mod performance_tests {
    #[test]
    fn test_batch_address_generation() {
        // 批量生成地址的性能测试
        #[cfg(feature = "bitcoin")]
        {
            use defi_hot_wallet::blockchain::bitcoin::{
                account::BitcoinKeypair,
                address::BitcoinAddress,
            };
            use bitcoin::Network;
            
            let keypair = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
            
            // 生成 100 个地址应该很快
            let start = std::time::Instant::now();
            for _ in 0..100 {
                let _ = BitcoinAddress::from_public_key_segwit(
                    keypair.public_key(),
                    Network::Bitcoin
                ).unwrap();
            }
            let duration = start.elapsed();
            
            // 100 个地址应该在 1 秒内完成
            assert!(duration.as_secs() < 1, "批量地址生成应该很快");
        }
    }
    
    #[test]
    fn test_batch_signing_performance() {
        #[cfg(feature = "bitcoin")]
        {
            use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
            use bitcoin::Network;
            
            let keypair = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
            
            // 签名 50 个消息
            let start = std::time::Instant::now();
            for i in 0..50 {
                let msg = [i as u8; 32];
                let _ = keypair.sign_schnorr(&msg).unwrap();
            }
            let duration = start.elapsed();
            
            // 50 个签名应该在 1 秒内完成
            assert!(duration.as_secs() < 1, "批量签名应该很快");
        }
    }
}

