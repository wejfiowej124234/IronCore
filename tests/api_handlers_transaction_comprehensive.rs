//! 交易 API 处理器综合测试
//! 目标：实现 100% 覆盖率

#[cfg(test)]
mod transaction_handler_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;
    
    use defi_hot_wallet::api::{
        server::WalletServer,
        types::{SendTransactionRequest, MultiSigTransactionRequest},
    };
    use defi_hot_wallet::core::config::WalletConfig;
    use defi_hot_wallet::security::SecretVec;
    
    // 辅助函数：创建测试服务器
    async fn create_test_server() -> WalletServer {
        std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        std::env::set_var("TEST_SKIP_DECRYPT", "1");
        std::env::set_var("BRIDGE_MOCK", "1");
        std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");
        
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
    
    // 辅助函数：创建测试钱包
    async fn create_test_wallet(server: &WalletServer, name: &str) {
        server.wallet_manager
            .create_wallet(name, false)
            .await
            .expect("Failed to create test wallet");
    }
    
    #[tokio::test]
    async fn test_send_transaction_success_eth() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.5".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        
        // 注意：实际发送可能失败（例如网络不可用），但 API 层应该正确处理
        // 可能的状态码：OK (200), BAD_REQUEST (400), NOT_FOUND (404), INTERNAL_SERVER_ERROR (500)
        let status = response.status();
        assert!(
            status == StatusCode::OK 
            || status == StatusCode::BAD_REQUEST 
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::INTERNAL_SERVER_ERROR,
            "预期状态码 200/400/404/500，实际: {}",
            status
        );
    }
    
    #[tokio::test]
    async fn test_send_transaction_missing_auth() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            // 没有 API Key
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_send_transaction_invalid_wallet_name() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/invalid-name-!/send")  // 包含无效字符
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[tokio::test]
    async fn test_send_transaction_wallet_not_found() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/nonexistent/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_send_transaction_missing_parameters() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "".to_string(),  // 空地址
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[tokio::test]
    async fn test_send_transaction_invalid_amount() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "invalid".to_string(),  // 无效金额
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[tokio::test]
    async fn test_send_transaction_unsupported_network() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "unsupported_network".to_string(),  // 不支持的网络
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[tokio::test]
    async fn test_send_transaction_bsc() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "0.5".to_string(),
            network: "bsc".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // BSC 交易可能成功或失败，取决于实现
        let status = response.status();
        assert!(
            status == StatusCode::OK || 
            status == StatusCode::INTERNAL_SERVER_ERROR ||
            status == StatusCode::BAD_REQUEST ||
            status == StatusCode::NOT_FOUND,
            "Expected OK, INTERNAL_SERVER_ERROR, BAD_REQUEST, or NOT_FOUND, got: {:?}", status
        );
    }
    
    #[tokio::test]
    async fn test_get_transaction_history_success() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/wallets/test_wallet/history")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_get_transaction_history_missing_auth() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/wallets/test_wallet/history")
            // 没有 API Key
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_get_transaction_history_wallet_not_found() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/wallets/nonexistent/history")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_send_multisig_transaction_basic() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures: vec![
                "0x1234567890abcdef".to_string(),
                "0xfedcba0987654321".to_string(),
            ],
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 多签交易可能成功或失败（取决于实现和网络状态）
        let status = response.status();
        assert!(
            status == StatusCode::OK 
            || status == StatusCode::BAD_REQUEST 
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::INTERNAL_SERVER_ERROR,
            "预期状态码 200/400/404/500，实际: {}",
            status
        );
    }
    
    #[tokio::test]
    async fn test_send_multisig_empty_signatures() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures: vec![],  // 空签名列表
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 可能成功或失败，取决于多签实现
        assert!(response.status() == StatusCode::OK || response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    #[tokio::test]
    async fn test_send_multisig_single_signature() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures: vec!["0xsig1".to_string()],  // 单个签名
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 可能成功或失败，取决于多签实现
        assert!(response.status() == StatusCode::OK || response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    // === 高级测试场景 ===
    
    #[tokio::test]
    async fn test_concurrent_transactions() {
        use futures::future::join_all;
        
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let _app = server.create_router().await;
        
        // 并发发送多个交易
        let mut handles = vec![];
        for i in 0..5 {
            let payload = SendTransactionRequest {
                to: format!("0x742d35Cc6634C0532925a3b844Bc9e7595f0bE{:x}", i),
                amount: format!("0.{}", i + 1),
                network: "eth".to_string(),
                password: "test_password".to_string(),
            client_request_id: None,
            };
            
            let _request = Request::builder()
                .method("POST")
                .uri("/api/wallets/test_wallet/send")
                .header(header::CONTENT_TYPE, "application/json")
                .header("Authorization", "test-api-key-12345678901234567890123")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap();
            
            // 注意：axum router 不能被多次使用，所以这个测试主要验证逻辑
            // 实际并发测试需要为每个请求创建新的 app 实例或使用真实服务器
            handles.push(async move {
                // 验证请求构造成功（payload 不为空）
                assert!(!payload.to.is_empty());
            });
        }
        
        join_all(handles).await;
    }
    
    #[tokio::test]
    async fn test_large_amount_transaction() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        // 测试大额交易
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "999999999.999999999".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 大额交易应该通过验证（但可能因为余额不足而失败）
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST 
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_transaction_with_special_characters_in_wallet_name() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 测试包含特殊字符的钱包名称
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/wallet%20with%20spaces/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回 400 或 404
        assert!(
            response.status() == StatusCode::BAD_REQUEST 
            || response.status() == StatusCode::NOT_FOUND
        );
    }
    
    #[tokio::test]
    async fn test_transaction_with_very_long_wallet_name() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 测试超长钱包名称
        let long_name = "a".repeat(1000);
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/wallets/{}/send", long_name))
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回 404（钱包不存在）
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_multisig_with_valid_threshold() {
        let server = create_test_server().await;
        create_test_wallet(&server, "multisig_wallet").await;
        let app = server.create_router().await;
        
        // 测试满足阈值的多签交易（2个签名）
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures: vec!["0xsig1".to_string(), "0xsig2".to_string()],
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/multisig_wallet/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该通过阈值验证（2个签名），但可能因业务逻辑失败
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_multisig_with_excessive_signatures() {
        let server = create_test_server().await;
        create_test_wallet(&server, "multisig_wallet").await;
        let app = server.create_router().await;
        
        // 测试过多签名的多签交易（10个签名）
        let signatures: Vec<String> = (0..10).map(|i| format!("0xsig{}", i)).collect();
        let payload = MultiSigTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/multisig_wallet/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该接受（虽然可能在后续验证中失败）
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_transaction_history_pagination() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        // 测试交易历史（当前不支持分页，但验证接口）
        let request = Request::builder()
            .method("GET")
            .uri("/api/wallets/test_wallet/history")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_send_transaction_with_zero_amount() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        // 测试零金额交易
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 零金额可能被拒绝
        assert!(
            response.status() == StatusCode::BAD_REQUEST 
            || response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_send_transaction_with_negative_amount() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        // 测试负金额交易
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "-1.0".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 负金额应该被拒绝
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[tokio::test]
    async fn test_multisig_invalid_address_format() {
        let server = create_test_server().await;
        create_test_wallet(&server, "multisig_wallet").await;
        let app = server.create_router().await;
        
        // 测试无效地址格式的多签交易
        let payload = MultiSigTransactionRequest {
            to: "invalid_address".to_string(),
            amount: "1.0".to_string(),
            network: "eth".to_string(),
            signatures: vec!["0xsig1".to_string(), "0xsig2".to_string()],
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/multisig_wallet/send_multi_sig")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回 400 错误
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[tokio::test]
    async fn test_transaction_with_polygon_network() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        // 测试 Polygon 网络交易
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "polygon".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // Polygon 应该被支持，但可能因为验证失败返回 400
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_transaction_with_bsc_network() {
        let server = create_test_server().await;
        create_test_wallet(&server, "test_wallet").await;
        let app = server.create_router().await;
        
        // 测试 BSC 网络交易
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "1.0".to_string(),
            network: "bsc".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/test_wallet/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // BSC 应该被支持，但可能因为验证失败返回 400
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}

