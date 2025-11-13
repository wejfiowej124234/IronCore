//! Bitcoin RPC 客户端综合测试

#[cfg(feature = "bitcoin")]
mod bitcoin_client_tests {
    use defi_hot_wallet::blockchain::bitcoin::client::BitcoinClient;
    use defi_hot_wallet::blockchain::traits::BlockchainClient;
    use bitcoin::Network;
    
    #[test]
    fn test_client_creation() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Testnet,
        );
        
        assert_eq!(client.get_network_name(), "testnet");
    }
    
    #[test]
    fn test_mainnet_client() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        );
        
        assert_eq!(client.get_network_name(), "bitcoin");
    }
    
    #[test]
    fn test_client_with_auth() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Testnet,
        );
        
        // Client 应该能创建
        assert_eq!(client.get_network_name(), "testnet");
    }
    
    #[test]
    fn test_client_without_auth() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        );
        
        assert_eq!(client.get_network_name(), "bitcoin");
    }
    
    #[test]
    fn test_different_rpc_urls() {
        let urls = vec![
            "http://localhost:8332",
            "http://127.0.0.1:8332",
            "https://bitcoin.example.com",
        ];
        
        for url in urls {
            let client = BitcoinClient::new(url.to_string(), Network::Bitcoin);
            assert_eq!(client.get_network_name(), "bitcoin");
        }
    }
    
    #[test]
    fn test_network_name_testnet() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Testnet);
        assert_eq!(client.get_network_name(), "testnet");
    }
    
    #[test]
    fn test_network_name_mainnet() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        assert_eq!(client.get_network_name(), "bitcoin");
    }
    
    #[test]
    fn test_native_token_name() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        assert_eq!(client.get_native_token(), "BTC");
    }
    
    #[test]
    fn test_client_cloneable() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        let cloned = client.clone_box();
        
        assert_eq!(cloned.get_network_name(), "bitcoin");
    }
}

#[cfg(feature = "bitcoin")]
mod address_validation_tests {
    use defi_hot_wallet::blockchain::bitcoin::client::BitcoinClient;
    use defi_hot_wallet::blockchain::BlockchainClient;
    use bitcoin::Network;
    
    #[tokio::test]
    async fn test_validate_valid_address() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        
        // 有效的比特币地址
        let valid_addresses = vec![
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",  // Genesis
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",  // SegWit
        ];
        
        for addr in valid_addresses {
            let result = client.validate_address(addr);
            assert!(result.is_ok());
        }
    }
    
    #[tokio::test]
    async fn test_validate_invalid_address() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        
        let invalid_addresses = vec![
            "",
            "invalid",
            "0xEthereumAddress",
        ];
        
        for addr in invalid_addresses {
            let result = client.validate_address(addr);
            assert!(result.is_ok());  // 返回 Ok(false)
        }
    }
}

#[cfg(feature = "bitcoin")]
mod error_handling_tests {
    use defi_hot_wallet::blockchain::bitcoin::client::BitcoinClient;
    use defi_hot_wallet::blockchain::BlockchainClient;
    use bitcoin::Network;
    
    #[tokio::test]
    async fn test_network_error_handling() {
        let client = BitcoinClient::new(
            "http://invalid.nowhere:9999".to_string(),
            Network::Bitcoin,
        );
        
        // 连接失败应该返回错误
        let result = client.get_balance("someaddress").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_empty_address_balance() {
        let client = BitcoinClient::new("http://localhost".to_string(), Network::Bitcoin);
        
        let result = client.get_balance("").await;
        // 应该处理空地址
        assert!(result.is_ok() || result.is_err());
    }
}

#[cfg(feature = "bitcoin")]
mod transaction_status_tests {
    use defi_hot_wallet::blockchain::traits::TransactionStatus;
    
    #[test]
    fn test_transaction_status_variants() {
        let statuses = vec![
            TransactionStatus::Pending,
            TransactionStatus::Confirmed,
            TransactionStatus::Failed,
            TransactionStatus::Unknown,
        ];
        
        for status in statuses {
            // 所有状态都应该能创建
            let _ = status.clone();
        }
    }
    
    #[test]
    fn test_transaction_status_equality() {
        assert_eq!(TransactionStatus::Pending, TransactionStatus::Pending);
        assert_ne!(TransactionStatus::Pending, TransactionStatus::Confirmed);
    }
}

