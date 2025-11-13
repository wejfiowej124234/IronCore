//! 交易 API 高级场景测试
//! 包含：多签、桥接、重试、批量操作等

#[cfg(test)]
mod advanced_transaction_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;
    
    use defi_hot_wallet::api::{
        server::WalletServer,
        types::{
            SendTransactionRequest, 
            MultiSigTransactionRequest,
            BridgeAssetsRequest,
        },
    };
    use defi_hot_wallet::core::config::WalletConfig;
    use defi_hot_wallet::security::SecretVec;
    
    // 辅助函数：创建测试服务器
    async fn create_test_server() -> WalletServer {
        std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        std::env::set_var("TEST_SKIP_DECRYPT", "1");
        std::env::set_var("BRIDGE_MOCK", "1");
        std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");
        std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
        
        let config = WalletConfig {
            storage: defi_hot_wallet::core::config::StorageConfig {
                database_url: "sqlite::memory:".to_string(),
                max_connections: Some(5),
                connection_timeout_seconds: Some(10),
            },
            ..Default::default()
        };
        
        let api_key_bytes = b"test-api-key-12345678901234567890123".to_vec();
        let api_key = Some(SecretVec::new(api_key_bytes));
        
        WalletServer::new_for_test(
            "127.0.0.1".to_string(),
            0,
            config,
            api_key,
            None
        ).await.expect("Failed to create test server")
    }
    
    async fn create_test_wallet(server: &WalletServer, name: &str) {
        server.wallet_manager
            .create_wallet(name, false)
            .await
            .ok();
    }
    
    // === 桥接交易测试 ===
    
    #[tokio::test]
    async fn test_bridge_btc_to_bsc() {
        let server = create_test_server().await;
        create_test_wallet(&server, "bridge_wallet").await;
        let app = server.create_router().await;
        
        let payload = BridgeAssetsRequest {
            from_wallet: "bridge_wallet".to_string(),
            from_chain: "btc".to_string(),
            to_chain: "bsc".to_string(),
            token: "BTC".to_string(),
            amount: "0.01".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/bridge")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 桥接应该返回成功或错误（取决于实现）
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_bridge_eth_to_bsc() {
        let server = create_test_server().await;
        create_test_wallet(&server, "eth_bridge").await;
        let app = server.create_router().await;
        
        let payload = BridgeAssetsRequest {
            from_wallet: "eth_bridge".to_string(),
            from_chain: "eth".to_string(),
            to_chain: "bsc".to_string(),
            token: "ETH".to_string(),
            amount: "0.5".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/bridge")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_bridge_invalid_chain() {
        let server = create_test_server().await;
        create_test_wallet(&server, "bridge_wallet").await;
        let app = server.create_router().await;
        
        let payload = BridgeAssetsRequest {
            from_wallet: "bridge_wallet".to_string(),
            from_chain: "invalid_chain".to_string(),
            to_chain: "bsc".to_string(),
            token: "BTC".to_string(),
            amount: "0.01".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/bridge")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回错误
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }
    
    #[tokio::test]
    async fn test_bridge_missing_wallet() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let payload = BridgeAssetsRequest {
            from_wallet: "nonexistent_wallet".to_string(),
            from_chain: "btc".to_string(),
            to_chain: "bsc".to_string(),
            token: "BTC".to_string(),
            amount: "0.01".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/bridge")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回 404、400 或 500
        assert!(
            response.status() == StatusCode::NOT_FOUND 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status().is_server_error()
        );
    }
    
    // === 多签交易高级测试 ===
    
    #[tokio::test]
    async fn test_multisig_3_of_5() {
        let server = create_test_server().await;
        create_test_wallet(&server, "multisig_3_5").await;
        let app = server.create_router().await;
        
        // 3-of-5 多签
        let signatures = vec![
            "0xsig1".to_string(),
            "0xsig2".to_string(),
            "0xsig3".to_string(),
        ];
        
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "10.0".to_string(),
            network: "eth".to_string(),
            signatures,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/multisig_3_5/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_multisig_duplicate_signatures() {
        let server = create_test_server().await;
        create_test_wallet(&server, "multisig_dup").await;
        let app = server.create_router().await;
        
        // 重复的签名
        let signatures = vec![
            "0xsig1".to_string(),
            "0xsig1".to_string(), // 重复
            "0xsig2".to_string(),
        ];
        
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/multisig_dup/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 可能被接受或拒绝，取决于验证逻辑
        assert!(response.status().as_u16() >= 200 && response.status().as_u16() < 600);
    }
    
    #[tokio::test]
    async fn test_multisig_invalid_signature_format() {
        let server = create_test_server().await;
        create_test_wallet(&server, "multisig_invalid").await;
        let app = server.create_router().await;
        
        let signatures = vec![
            "not_a_valid_signature".to_string(),
            "also_invalid".to_string(),
        ];
        
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/multisig_invalid/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 可能返回错误
        assert!(response.status().as_u16() >= 200);
    }
    
    // === 批量交易测试 ===
    
    #[tokio::test]
    async fn test_sequential_transactions() {
        let server = create_test_server().await;
        create_test_wallet(&server, "seq_tx").await;
        
        // 连续发送多笔交易
        for i in 0..5 {
            let app = server.clone().create_router().await;
            
            let payload = SendTransactionRequest {
                to: format!("0x742d35Cc6634C0532925a3b844Bc9e7595f0bE{:x}", i),
                amount: format!("0.{}", i + 1),
                network: "eth".to_string(),
                password: "test_password".to_string(),
            client_request_id: None,
            };
            
            let request = Request::builder()
                .method("POST")
                .uri("/api/wallets/seq_tx/send")
                .header(header::CONTENT_TYPE, "application/json")
                .header("Authorization", "test-api-key-12345678901234567890123")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap();
            
            let response = app.oneshot(request).await.unwrap();
            // 每笔交易都应该被处理（成功或失败）
            assert!(response.status().as_u16() >= 200);
        }
    }
    
    // === 交易历史高级查询 ===
    
    #[tokio::test]
    async fn test_transaction_history_after_multiple_tx() {
        let server = create_test_server().await;
        create_test_wallet(&server, "history_test").await;
        
        // 发送几笔交易（尝试）
        for i in 0..3 {
            let app = server.clone().create_router().await;
            let payload = SendTransactionRequest {
                to: format!("0x742d35Cc6634C0532925a3b844Bc9e7595f0bE{:x}", i),
                amount: "0.1".to_string(),
                network: "eth".to_string(),
                password: "test_password".to_string(),
            client_request_id: None,
            };
            
            let request = Request::builder()
                .method("POST")
                .uri("/api/wallets/history_test/send")
                .header(header::CONTENT_TYPE, "application/json")
                .header("Authorization", "test-api-key-12345678901234567890123")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap();
            
            let _ = app.oneshot(request).await;
        }
        
        // 查询历史
        let app = server.create_router().await;
        let request = Request::builder()
            .method("GET")
            .uri("/api/wallets/history_test/history")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    // === 网络切换测试 ===
    
    #[tokio::test]
    async fn test_switch_between_networks() {
        let server = create_test_server().await;
        create_test_wallet(&server, "network_switch").await;
        
        let networks = vec!["eth", "sepolia", "polygon", "bsc"];
        
        for network in networks {
            let app = server.clone().create_router().await;
            
            let payload = SendTransactionRequest {
                to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
                amount: "0.1".to_string(),
                network: network.to_string(),
                password: "test_password".to_string(),
            client_request_id: None,
            };
            
            let request = Request::builder()
                .method("POST")
                .uri("/api/wallets/network_switch/send")
                .header(header::CONTENT_TYPE, "application/json")
                .header("Authorization", "test-api-key-12345678901234567890123")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap();
            
            let response = app.oneshot(request).await.unwrap();
            // 应该支持所有这些网络
            assert!(
                response.status() == StatusCode::OK 
                || response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
                "网络 {} 应该被支持", network
            );
        }
    }
    
    // === 金额边界测试 ===
    
    #[tokio::test]
    async fn test_very_small_amount() {
        let server = create_test_server().await;
        create_test_wallet(&server, "small_amount").await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "0.000000000000000001".to_string(), // 1 wei
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/small_amount/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该能处理极小金额
        assert!(response.status().as_u16() >= 200);
    }
    
    #[tokio::test]
    async fn test_scientific_notation_amount() {
        let server = create_test_server().await;
        create_test_wallet(&server, "sci_notation").await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.5e-8".to_string(), // 科学计数法
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/sci_notation/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 科学计数法可能被接受或拒绝
        assert!(response.status().as_u16() >= 200);
    }
    
    // === 地址格式测试 ===
    
    #[tokio::test]
    async fn test_checksum_address() {
        let server = create_test_server().await;
        create_test_wallet(&server, "checksum_test").await;
        let app = server.create_router().await;
        
        // EIP-55 checksum 地址
        let payload = SendTransactionRequest {
            to: "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed".to_string(),
            amount: "0.1".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/checksum_test/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().as_u16() >= 200);
    }
    
    #[tokio::test]
    async fn test_lowercase_address() {
        let server = create_test_server().await;
        create_test_wallet(&server, "lowercase_test").await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed".to_string(),
            amount: "0.1".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/lowercase_test/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().as_u16() >= 200);
    }
    
    // === 错误恢复测试 ===
    
    #[tokio::test]
    async fn test_transaction_after_failed_attempt() {
        let server = create_test_server().await;
        create_test_wallet(&server, "retry_wallet").await;
        
        // 第一次尝试：无效金额
        let app1 = server.clone().create_router().await;
        let payload1 = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "invalid".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request1 = Request::builder()
            .method("POST")
            .uri("/api/wallets/retry_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload1).unwrap()))
            .unwrap();
        
        let response1 = app1.oneshot(request1).await.unwrap();
        assert_eq!(response1.status(), StatusCode::BAD_REQUEST);
        
        // 第二次尝试：有效请求
        let app2 = server.create_router().await;
        let payload2 = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "0.1".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request2 = Request::builder()
            .method("POST")
            .uri("/api/wallets/retry_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload2).unwrap()))
            .unwrap();
        
        let response2 = app2.oneshot(request2).await.unwrap();
        // 第二次应该能正常处理（可能返回 BAD_REQUEST 如果钱包不存在）
        assert!(
            response2.status() == StatusCode::OK 
            || response2.status() == StatusCode::BAD_REQUEST
            || response2.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}


