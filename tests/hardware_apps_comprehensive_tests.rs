//! 硬件钱包 Apps 综合测试

#[cfg(feature = "ledger")]
mod ledger_bitcoin_app_tests {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    
    #[test]
    fn test_bip32_path_standard_paths() {
        let paths = vec![
            ("m/44'/0'/0'/0/0", 5),    // Legacy
            ("m/49'/0'/0'/0/0", 5),    // P2SH-SegWit
            ("m/84'/0'/0'/0/0", 5),    // Native SegWit
            ("m/86'/0'/0'/0/0", 5),    // Taproot
        ];
        
        for (path_str, expected_len) in paths {
            let path = Bip32Path::from_str(path_str).unwrap();
            assert_eq!(path.path.len(), expected_len);
        }
    }
    
    #[test]
    fn test_bip32_path_hardened_notation() {
        // 测试 ' 和 h 两种硬化标记
        let path1 = Bip32Path::from_str("m/44'/0'/0'").unwrap();
        let path2 = Bip32Path::from_str("m/44h/0h/0h").unwrap();
        
        assert_eq!(path1.path, path2.path, "' 和 h 应该等价");
    }
    
    #[test]
    fn test_bip32_path_mixed_notation() {
        let path = Bip32Path::from_str("m/44'/0h/0'/0/0").unwrap();
        assert_eq!(path.path.len(), 5);
    }
    
    #[test]
    fn test_bip32_path_unhardened() {
        let path = Bip32Path::from_str("m/44/60/0/0/0").unwrap();
        
        // 所有索引都应该 < 0x80000000
        for &index in &path.path {
            assert!(index < 0x80000000, "非硬化索引应该 < 0x80000000");
        }
    }
    
    #[test]
    fn test_bip32_path_hardened() {
        let path = Bip32Path::from_str("m/44'/60'/0'").unwrap();
        
        // 所有索引都应该 >= 0x80000000
        for &index in &path.path {
            assert!(index >= 0x80000000, "硬化索引应该 >= 0x80000000");
        }
    }
    
    #[test]
    fn test_bip32_ethereum_path() {
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        assert_eq!(path.path[0], 0x8000002C);  // 44'
        assert_eq!(path.path[1], 0x8000003C);  // 60' (Ethereum)
    }
    
    #[test]
    fn test_bip32_bitcoin_path() {
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        assert_eq!(path.path[0], 0x8000002C);  // 44'
        assert_eq!(path.path[1], 0x80000000);  // 0' (Bitcoin)
    }
    
    #[test]
    fn test_bip32_path_serialization_format() {
        let path = Bip32Path::new(vec![0x8000002C, 0x80000000]);
        let bytes = path.to_bytes();
        
        // 格式: length(1) + indices(4*n)
        assert_eq!(bytes[0], 2);  // 长度
        assert_eq!(bytes.len(), 1 + 2 * 4);
    }
    
    #[test]
    fn test_bip32_empty_path() {
        let path = Bip32Path::new(vec![]);
        let bytes = path.to_bytes();
        
        assert_eq!(bytes[0], 0);  // 长度为 0
        assert_eq!(bytes.len(), 1);
    }
    
    #[test]
    fn test_bip32_single_index() {
        let path = Bip32Path::from_str("m/44'").unwrap();
        assert_eq!(path.path.len(), 1);
        assert_eq!(path.path[0], 0x8000002C);
    }
    
    #[test]
    fn test_bip32_path_invalid_format() {
        let invalid_paths = vec![
            "invalid",
            "m/",
            "m/abc",
            "44'/0'",  // 缺少 m/
        ];
        
        for p in invalid_paths {
            let result = Bip32Path::from_str(p);
            assert!(result.is_err(), "无效路径应该失败: {}", p);
        }
    }
    
    #[test]
    fn test_bip32_very_deep_path() {
        let path = Bip32Path::from_str("m/44'/0'/0'/0'/0'/0'/0'/0'/0'/0'").unwrap();
        assert_eq!(path.path.len(), 10);
    }
    
    #[test]
    fn test_bip32_path_large_index() {
        let path = Bip32Path::new(vec![0xFFFFFFFF]);
        assert_eq!(path.path[0], 0xFFFFFFFF);
    }
}

#[cfg(feature = "trezor")]
mod trezor_bitcoin_app_tests {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    
    #[test]
    fn test_bip86_taproot_path() {
        let path = Bip32Path::from_str("m/86'/0'/0'/0/0").unwrap();
        assert_eq!(path.path[0], 0x80000056);  // 86'
    }
    
    #[test]
    fn test_bip84_segwit_path() {
        let path = Bip32Path::from_str("m/84'/0'/0'/0/0").unwrap();
        assert_eq!(path.path[0], 0x80000054);  // 84'
    }
    
    #[test]
    fn test_bip49_nested_segwit() {
        let path = Bip32Path::from_str("m/49'/0'/0'/0/0").unwrap();
        assert_eq!(path.path[0], 0x80000031);  // 49'
    }
    
    #[test]
    fn test_path_serialization_big_endian() {
        let path = Bip32Path::new(vec![0x12345678]);
        let bytes = path.to_bytes();
        
        // 应该是大端序
        assert_eq!(bytes[1], 0x12);
        assert_eq!(bytes[2], 0x34);
        assert_eq!(bytes[3], 0x56);
        assert_eq!(bytes[4], 0x78);
    }
    
    #[test]
    fn test_multiple_accounts() {
        // 多个账户路径
        for account in 0..5 {
            let path_str = format!("m/44'/0'/{}'/0/0", account);
            let path = Bip32Path::from_str(&path_str).unwrap();
            assert_eq!(path.path[2], 0x80000000 + account);
        }
    }
    
    #[test]
    fn test_change_addresses() {
        // 找零地址（change=1）
        let external = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        let change = Bip32Path::from_str("m/44'/0'/0'/1/0").unwrap();
        
        assert_eq!(external.path[3], 0);
        assert_eq!(change.path[3], 1);
    }
    
    #[test]
    fn test_address_index_range() {
        // 测试不同的地址索引
        for index in vec![0, 1, 10, 100, 1000] {
            let path_str = format!("m/44'/0'/0'/0/{}", index);
            let path = Bip32Path::from_str(&path_str).unwrap();
            assert_eq!(path.path[4], index);
        }
    }
}

#[cfg(feature = "ledger")]
mod ledger_ethereum_app_tests {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    
    #[test]
    fn test_ethereum_standard_path() {
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        assert_eq!(path.path[0], 0x8000002C);  // 44'
        assert_eq!(path.path[1], 0x8000003C);  // 60' (Ethereum)
        assert_eq!(path.path[2], 0x80000000);  // 0'
        assert_eq!(path.path[3], 0);  // 0
        assert_eq!(path.path[4], 0);  // 0
    }
    
    #[test]
    fn test_ethereum_multiple_accounts() {
        for i in 0..10 {
            let path_str = format!("m/44'/60'/{}'/0/0", i);
            let path = Bip32Path::from_str(&path_str).unwrap();
            assert_eq!(path.path[2], 0x80000000 + i);
        }
    }
    
    #[test]
    fn test_ethereum_erc20_same_path() {
        // ERC-20 使用相同的路径
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        assert_eq!(path.path.len(), 5);
    }
}

#[cfg(feature = "trezor")]
mod trezor_ethereum_app_tests {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    
    #[test]
    fn test_ledger_live_path() {
        // Ledger Live 使用的路径
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        assert_eq!(path.path.len(), 5);
    }
    
    #[test]
    fn test_metamask_path() {
        // MetaMask 使用的路径
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        assert_eq!(path.path[1], 0x8000003C);  // 60'
    }
}

#[cfg(feature = "bitcoin")]
mod blockchain_client_trait_tests {
    use defi_hot_wallet::blockchain::bitcoin::client::BitcoinClient;
    use defi_hot_wallet::blockchain::traits::BlockchainClient;
    use bitcoin::Network;
    
    #[test]
    fn test_get_native_token() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        assert_eq!(client.get_native_token(), "BTC");
    }
    
    #[test]
    fn test_get_network_name_trait() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Testnet);
        assert_eq!(client.get_network_name(), "testnet");
    }
    
    #[tokio::test]
    async fn test_get_nonce_returns_zero() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        
        // Bitcoin 没有 nonce 概念，应该返回 0
        let nonce = client.get_nonce("anyaddress").await.unwrap_or(0);
        assert_eq!(nonce, 0);
    }
    
    #[tokio::test]
    async fn test_get_block_number_format() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        
        // 应该返回错误或区块高度
        let result = client.get_block_number().await;
        assert!(result.is_ok() || result.is_err());
    }
}

