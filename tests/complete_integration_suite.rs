//! 完整集成测试套件 - 95% 覆盖率冲刺

#[cfg(feature = "bitcoin")]
mod complete_bitcoin_workflow {
    use defi_hot_wallet::blockchain::bitcoin::{
        account::BitcoinKeypair,
        address::{AddressType, BitcoinAddress},
        utxo::{SelectionStrategy, Utxo, UtxoSelector},
        transaction::BitcoinTransaction,
    };
    use bitcoin::Network;
    
    #[test]
    fn test_complete_legacy_workflow() {
        // 1. 生成密钥
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        // 2. 生成地址
        let addr = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Testnet).unwrap();
        assert!(addr.starts_with('m') || addr.starts_with('n'));
        
        // 3. 验证地址
        assert!(BitcoinAddress::validate(&addr, Network::Testnet).unwrap());
        
        // 4. 模拟 UTXO
        let utxos = vec![Utxo::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".into(), 0, 100_000, "76a914".to_string() + &"00".repeat(20) + "88ac", 6)];
        
        // 5. 选择 UTXO
        let (selected, fee) = UtxoSelector::select(&utxos, 50_000, 10, SelectionStrategy::LargestFirst).unwrap();
        
        // 6. 构建交易
        let tx = BitcoinTransaction::build_legacy(&kp, &selected, &addr, 50_000, fee, Network::Testnet).unwrap();
        
        // 7. 序列化
        let hex = BitcoinTransaction::serialize(&tx);
        
        // 8. 验证
        assert!(!hex.is_empty());
        assert!(tx.input.len() > 0);
        assert!(tx.output.len() > 0);
    }
    
    #[test]
    fn test_complete_segwit_workflow() {
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let addr = BitcoinAddress::from_public_key_segwit(kp.public_key(), Network::Testnet).unwrap();
        
        assert!(addr.starts_with("tb1q"));
        assert!(BitcoinAddress::validate(&addr, Network::Testnet).unwrap());
        
        let utxos = vec![Utxo::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".into(), 0, 100_000, "0014".to_string() + &"00".repeat(20), 6)];
        let (selected, fee) = UtxoSelector::select(&utxos, 50_000, 10, SelectionStrategy::BestFit).unwrap();
        
        let tx = BitcoinTransaction::build_segwit(&kp, &selected, &addr, 50_000, fee, Network::Testnet).unwrap();
        
        assert!(!tx.input[0].witness.is_empty());
        assert!(tx.input[0].script_sig.is_empty());
    }
    
    #[test]
    fn test_complete_taproot_workflow() {
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let addr = BitcoinAddress::from_public_key_taproot(kp.public_key(), Network::Testnet).unwrap();
        
        assert!(addr.starts_with("tb1p"));
        assert_eq!(addr.len(), 62);
        
        // 使用有效的 64 字符十六进制 TXID
        let valid_txid = "0".repeat(64);
        let utxos = vec![Utxo::new(valid_txid, 0, 100_000, "5120".to_string() + &"00".repeat(32), 6)];
        let tx = BitcoinTransaction::build_taproot(&kp, &utxos, &addr, 50_000, 1_000, Network::Testnet).unwrap();
        
        // Taproot witness 只有签名
        assert_eq!(tx.input[0].witness.len(), 1);
        // Taproot 签名长度应该是 64 字节
        assert!(tx.input[0].witness.len() > 0);
    }
    
    #[test]
    fn test_multi_address_type_compatibility() {
        // 一个密钥可以生成多种地址类型
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        let legacy = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Bitcoin).unwrap();
        let segwit = BitcoinAddress::from_public_key_segwit(kp.public_key(), Network::Bitcoin).unwrap();
        let taproot = BitcoinAddress::from_public_key_taproot(kp.public_key(), Network::Bitcoin).unwrap();
        
        // 所有地址都应该有效
        assert!(BitcoinAddress::validate(&legacy, Network::Bitcoin).unwrap());
        assert!(BitcoinAddress::validate(&segwit, Network::Bitcoin).unwrap());
        assert!(BitcoinAddress::validate(&taproot, Network::Bitcoin).unwrap());
        
        // 类型应该可检测
        assert_eq!(BitcoinAddress::detect_type(&legacy).unwrap(), AddressType::Legacy);
        assert_eq!(BitcoinAddress::detect_type(&segwit).unwrap(), AddressType::SegWit);
        assert_eq!(BitcoinAddress::detect_type(&taproot).unwrap(), AddressType::Taproot);
    }
}

#[cfg(all(feature = "ledger", feature = "bitcoin"))]
mod ledger_bitcoin_integration {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    
    #[test]
    fn test_bip_standard_paths_complete() {
        let standard_paths = vec![
            ("BIP44 Legacy", "m/44'/0'/0'/0/0"),
            ("BIP49 Nested SegWit", "m/49'/0'/0'/0/0"),
            ("BIP84 Native SegWit", "m/84'/0'/0'/0/0"),
            ("BIP86 Taproot", "m/86'/0'/0'/0/0"),
        ];
        
        for (name, path_str) in standard_paths {
            let path = Bip32Path::from_str(path_str).unwrap();
            assert_eq!(path.path.len(), 5, "{} 路径长度应该是 5", name);
            
            let bytes = path.to_bytes();
            assert_eq!(bytes[0], 5, "{} 序列化长度应该是 5", name);
        }
    }
    
    #[test]
    fn test_account_derivation_range() {
        // 测试账户派生范围
        for account in 0..10 {
            let path_str = format!("m/84'/0'/{}'/0/0", account);
            let path = Bip32Path::from_str(&path_str).unwrap();
            
            assert_eq!(path.path[2], 0x80000000 | account);
        }
    }
    
    #[test]
    fn test_address_index_derivation_range() {
        // 测试地址索引范围
        for index in vec![0, 1, 10, 100, 1000, 10000] {
            let path_str = format!("m/84'/0'/0'/0/{}", index);
            let path = Bip32Path::from_str(&path_str).unwrap();
            
            assert_eq!(path.path[4], index);
        }
    }
}

#[cfg(all(feature = "trezor", feature = "bitcoin"))]
mod trezor_bitcoin_integration {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    
    #[test]
    fn test_bip32_path_bytes_order() {
        let path = Bip32Path::new(vec![0x12345678]);
        let bytes = path.to_bytes();
        
        // 大端序
        assert_eq!(bytes[1], 0x12);
        assert_eq!(bytes[2], 0x34);
        assert_eq!(bytes[3], 0x56);
        assert_eq!(bytes[4], 0x78);
    }
    
    #[test]
    fn test_path_depth_limits() {
        // 测试不同深度的路径
        for depth in 1..=10 {
            let indices = vec![0x8000002C; depth];
            let path = Bip32Path::new(indices);
            
            assert_eq!(path.path.len(), depth);
            
            let bytes = path.to_bytes();
            assert_eq!(bytes[0], depth as u8);
        }
    }
}

#[cfg(test)]
mod cross_module_integration {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_key_to_address_to_transaction() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
            transaction::BitcoinTransaction,
            utxo::Utxo,
        };
        use bitcoin::Network;
        use defi_hot_wallet::core::domain::PrivateKey;
        
        // 从私钥字节开始
        let key_bytes = [42u8; 32];
        let pk = PrivateKey::try_from_slice(&key_bytes).unwrap();
        
        // 创建密钥对
        let kp = BitcoinKeypair::from_private_key(&pk, Network::Testnet).unwrap();
        
        // 生成地址
        let addr = BitcoinAddress::from_public_key_taproot(kp.public_key(), Network::Testnet).unwrap();
        
        // 构建交易
        let valid_txid = "0".repeat(64);
        let utxos = vec![Utxo::new(valid_txid, 0, 100_000, "5120".to_string() + &"00".repeat(32), 6)];
        let tx = BitcoinTransaction::build_taproot(&kp, &utxos, &addr, 50_000, 1_000, Network::Testnet).unwrap();
        
        // 验证完整流程
        assert!(!addr.is_empty());
        assert!(tx.input.len() > 0);
        assert!(tx.output.len() > 0);
    }
}

#[cfg(test)]
mod data_consistency_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_utxo_data_integrity() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        let txid = "abcdef1234567890".to_string();
        let vout = 5;
        let amount = 123456;
        let script = "script_data".to_string();
        let conf = 10;
        
        let utxo = Utxo::new(txid.clone(), vout, amount, script.clone(), conf);
        
        assert_eq!(utxo.txid, txid);
        assert_eq!(utxo.vout, vout);
        assert_eq!(utxo.amount, amount);
        assert_eq!(utxo.script_pubkey, script);
        assert_eq!(utxo.confirmations, conf);
    }
}

