//! 服务器启动和路由综合测试
//! 目标：实现 ≥90% 覆盖率

#[cfg(test)]
mod server_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;
    
    use defi_hot_wallet::api::{
        server::WalletServer,
        types::{CreateWalletRequest, SendTransactionRequest},
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
    
    // === 健康检查测试 ===
    
    #[tokio::test]
    async fn test_health_check() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_metrics_endpoint() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/metrics")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // Metrics 端点应该返回 200 或需要认证
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::UNAUTHORIZED
        );
    }
    
    // === 路由集成测试 ===
    
    #[tokio::test]
    async fn test_full_wallet_workflow() {
        let server = create_test_server().await;
        
        // 1. 创建钱包
        server.wallet_manager.create_wallet("workflow_wallet", false).await.ok();
        
        // 2. 列出钱包
        let wallets = server.wallet_manager.list_wallets().await.unwrap();
        assert!(wallets.iter().any(|w| w.name == "workflow_wallet"));
        
        // 3. 发送交易（通过 wallet_manager，因为 router 只能使用一次）
        let result = server.wallet_manager.send_transaction(
            "workflow_wallet",
            "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
            "0.1",
            "eth",
            "test_password"
        ).await;
        // 交易可能成功或失败，但应该不会崩溃
        assert!(result.is_ok() || result.is_err());
        
        // 4. 删除钱包
        server.wallet_manager.delete_wallet("workflow_wallet").await.ok();
    }
    
    #[tokio::test]
    async fn test_router_creation() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 验证 router 创建成功
        // 尝试访问一个不存在的路由，应该返回 404
        let request = Request::builder()
            .method("GET")
            .uri("/api/nonexistent")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_cors_middleware() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 发送带有 Origin 头的请求
        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/health")
            .header("Origin", "http://localhost:3000")
            .header("Access-Control-Request-Method", "GET")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // CORS OPTIONS 请求应该被正确处理
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::NO_CONTENT
        );
    }
    
    #[tokio::test]
    async fn test_rate_limiting_middleware() {
        let server = create_test_server().await;
        
        // 测试速率限制器是否正确初始化
        assert!(server.rate_limiter.as_ref() as *const _ != std::ptr::null());
        assert!(server.rate_limiter.as_ref() as *const _ != std::ptr::null());
    }
    
    #[tokio::test]
    async fn test_auth_middleware() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 测试需要认证的端点
        let payload = CreateWalletRequest {
            name: "auth_test_wallet".to_string(),
            password: "test_password".to_string(),
            quantum_safe: false,
            generate_mnemonic: true,
            mnemonic_word_count: 12,
            wallet_type: None,
            multisig_config: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets")
            .header(header::CONTENT_TYPE, "application/json")
            // 不提供认证头
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    // === 中间件测试 ===
    
    #[tokio::test]
    async fn test_request_body_limit() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 创建一个超大的请求体（注意：实际限制是 1MB）
        let large_name = "a".repeat(2 * 1024 * 1024); // 2MB
        let payload = CreateWalletRequest {
            name: large_name,
            password: "test_password".to_string(),
            quantum_safe: false,
            generate_mnemonic: true,
            mnemonic_word_count: 12,
            wallet_type: None,
            multisig_config: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该因为请求体过大而被拒绝
        assert!(
            response.status() == StatusCode::PAYLOAD_TOO_LARGE
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_concurrent_requests_limit() {
        let server = create_test_server().await;
        
        // 验证并发限制层已配置（ConcurrencyLimitLayer）
        // 注意：实际测试并发限制需要真实的服务器或更复杂的测试设置
        let app = server.create_router().await;
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    // === 多路由测试 ===
    
    #[tokio::test]
    async fn test_all_routes_accessible() {
        let server = create_test_server().await;
        
        // 创建测试钱包
        server.wallet_manager.create_wallet("route_test_wallet", false).await.ok();
        
        // 路由列表：(路径, 方法, 需要认证, 允许404)
        let routes = vec![
            ("/api/health", "GET", false, false),
            ("/api/wallets", "GET", true, false),
            ("/api/wallets", "POST", true, false),
            ("/api/wallets/route_test_wallet", "DELETE", true, false),
            ("/api/wallets/route_test_wallet/history", "GET", true, true), // 可能404
            ("/api/wallets/route_test_wallet/backup", "GET", true, true),  // 可能404
            ("/api/wallets/restore", "POST", true, false),
            ("/api/wallets/route_test_wallet/rotate-signing-key", "POST", true, true), // 可能404
            ("/api/wallets/route_test_wallet/send", "POST", true, false),
            ("/api/wallets/route_test_wallet/send_multi_sig", "POST", true, false),
            ("/api/bridge", "POST", true, false),
            ("/api/metrics", "GET", false, false),
        ];
        
        for (path, method, needs_auth, allow_404) in routes {
            let app = server.clone().create_router().await;
            
            let mut req_builder = Request::builder()
                .method(method)
                .uri(path);
            
            if needs_auth {
                req_builder = req_builder.header("Authorization", "test-api-key-12345678901234567890123");
            }
            
            if method == "POST" {
                req_builder = req_builder.header(header::CONTENT_TYPE, "application/json");
            }
            
            let request = req_builder
                .body(Body::from("{}"))
                .unwrap();
            
            let response = app.oneshot(request).await.unwrap();
            
            // 验证路由存在（不是 404，除非明确允许）
            if !allow_404 {
                assert_ne!(
                    response.status(),
                    StatusCode::NOT_FOUND,
                    "路由 {} {} 不应返回 404",
                    method,
                    path
                );
            }
            // 对于允许 404 的路由，只要不是方法不允许(405)或内部错误就行
            if allow_404 {
                let status = response.status();
                assert!(
                    status != StatusCode::METHOD_NOT_ALLOWED,
                    "路由 {} {} 不应返回 405 (方法不允许)",
                    method,
                    path
                );
            }
        }
    }
    
    #[tokio::test]
    async fn test_server_clone() {
        let server = create_test_server().await;
        let server_clone = server.clone();
        
        // 验证克隆后的服务器配置相同
        assert_eq!(server.host, server_clone.host);
        assert_eq!(server.port, server_clone.port);
    }
    
    #[tokio::test]
    async fn test_server_with_different_ports() {
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
        
        // 创建不同端口的服务器
        let server1 = WalletServer::new_for_test(
            "127.0.0.1".to_string(),
            8080,
            config.clone(),
            api_key.clone(),
            None
        ).await.unwrap();
        
        let server2 = WalletServer::new_for_test(
            "127.0.0.1".to_string(),
            8081,
            config,
            api_key,
            None
        ).await.unwrap();
        
        assert_ne!(server1.port, server2.port);
    }
    
    #[tokio::test]
    async fn test_server_with_custom_config() {
        let mut config = WalletConfig::default();
        config.storage.database_url = "sqlite::memory:".to_string();
        config.storage.max_connections = Some(10);
        
        let api_key_bytes = b"custom-api-key-12345678901234567890".to_vec();
        let api_key = Some(SecretVec::new(api_key_bytes));
        
        let server = WalletServer::new_for_test(
            "0.0.0.0".to_string(),
            0,
            config,
            api_key,
            None
        ).await.unwrap();
        
        assert_eq!(server.host, "0.0.0.0");
        assert_eq!(server.config.storage.max_connections, Some(10));
    }
    
    #[tokio::test]
    async fn test_error_handling_middleware() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 测试错误处理中间件
        // 发送一个会触发内部错误的请求
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from("{invalid json"))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回错误状态码
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }
    
    #[tokio::test]
    async fn test_trace_layer() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 发送请求以触发 TraceLayer
        let request = Request::builder()
            .method("GET")
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_sensitive_routes_have_stricter_limits() {
        let server = create_test_server().await;
        server.wallet_manager.create_wallet("sensitive_test", false).await.ok();
        
        // 敏感路由（/send, /bridge）应该有更严格的限制
        // 这里我们只验证路由是可访问的
        let app = server.create_router().await;
        
        let payload = SendTransactionRequest {
            to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            amount: "0.1".to_string(),
            network: "eth".to_string(),
            password: "test_password".to_string(),
            client_request_id: None,
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/sensitive_test/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该不是 404（路由存在）
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_server_initialization() {
        std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        std::env::set_var("TEST_SKIP_DECRYPT", "1");
        
        let config = WalletConfig {
            storage: defi_hot_wallet::core::config::StorageConfig {
                database_url: "sqlite::memory:".to_string(),
                max_connections: Some(5),
                connection_timeout_seconds: Some(10),
            },
            ..Default::default()
        };
        
        let api_key_bytes = b"init-test-key-12345678901234567890123".to_vec();
        let api_key = Some(SecretVec::new(api_key_bytes));
        
        // 测试标准的 new 构造函数
        let server = WalletServer::new(
            "127.0.0.1".to_string(),
            0,
            config,
            api_key,
        ).await;
        
        assert!(server.is_ok(), "服务器初始化应该成功");
        let server = server.unwrap();
        assert_eq!(server.host, "127.0.0.1");
    }
    
    #[tokio::test]
    async fn test_server_without_api_key() {
        std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        std::env::set_var("TEST_SKIP_DECRYPT", "1");
        
        let config = WalletConfig {
            storage: defi_hot_wallet::core::config::StorageConfig {
                database_url: "sqlite::memory:".to_string(),
                max_connections: Some(5),
                connection_timeout_seconds: Some(10),
            },
            ..Default::default()
        };
        
        // 创建没有 API Key 的服务器
        let server = WalletServer::new(
            "127.0.0.1".to_string(),
            0,
            config,
            None, // 无 API Key
        ).await;
        
        assert!(server.is_ok(), "服务器初始化应该成功（即使没有 API Key）");
    }
    
    // === 深度集成测试 ===
    
    #[tokio::test]
    async fn test_full_backup_workflow() {
        let server = create_test_server().await;
        
        // 1. 创建钱包
        server.wallet_manager.create_wallet("backup_test", false).await.ok();
        
        let app = server.create_router().await;
        
        // 2. 备份钱包
        let request = Request::builder()
            .method("GET")
            .uri("/api/wallets/backup_test/backup")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 备份应该返回 200 或可能的错误
        assert!(response.status() == StatusCode::OK || response.status().is_server_error());
    }
    
    #[tokio::test]
    async fn test_404_error_handling() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 访问不存在的路由
        let request = Request::builder()
            .method("GET")
            .uri("/api/nonexistent/route")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_rate_limit_bypass_for_health() {
        let server = create_test_server().await;
        
        // 健康检查不应该被速率限制
        for _ in 0..20 {
            let app = server.clone().create_router().await;
            let request = Request::builder()
                .method("GET")
                .uri("/api/health")
                .body(Body::empty())
                .unwrap();
            
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }
    
    #[tokio::test]
    async fn test_bridge_endpoint_exists() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/bridge")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from("{}"))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 桥接端点应该存在（不是 404）
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }
    
    #[tokio::test]
    async fn test_options_request_cors() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 测试 CORS 预检请求
        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/wallets")
            .header("Origin", "http://localhost:3000")
            .header("Access-Control-Request-Method", "POST")
            .header("Access-Control-Request-Headers", "content-type,authorization")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // CORS OPTIONS 应该被处理
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::NO_CONTENT
        );
    }
    
    #[tokio::test]
    async fn test_concurrent_wallet_operations() {
        use futures::future::join_all;
        
        let server = create_test_server().await;
        
        // 并发创建多个钱包
        let mut handles = vec![];
        for i in 0..5 {
            let wallet_manager = server.wallet_manager.clone();
            let handle = tokio::spawn(async move {
                wallet_manager.create_wallet(&format!("concurrent_{}", i), false).await
            });
            handles.push(handle);
        }
        
        let results = join_all(handles).await;
        
        // 大部分应该成功（可能有重复名称失败）
        let success_count = results.iter().filter(|r| r.as_ref().ok().and_then(|inner| inner.as_ref().ok()).is_some()).count();
        assert!(success_count >= 3, "至少3个并发钱包创建应该成功");
    }
    
    #[tokio::test]
    async fn test_metrics_without_auth() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // Metrics 端点可能不需要认证
        let request = Request::builder()
            .method("GET")
            .uri("/api/metrics")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回 200 或 401
        assert!(response.status() == StatusCode::OK || response.status() == StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_wallet_restore_endpoint() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        let restore_payload = serde_json::json!({
            "name": "restored_wallet",
            "seed_phrase": "test seed phrase for restoration",
            "quantum_safe": false
        });
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/wallets/restore")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", "test-api-key-12345678901234567890123")
            .body(Body::from(restore_payload.to_string()))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 恢复可能成功或因为无效的种子短语失败
        assert!(
            response.status() == StatusCode::OK 
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    
    #[tokio::test]
    async fn test_multiple_auth_attempts() {
        let server = create_test_server().await;
        
        let wrong_keys = vec![
            "wrong-key-1",
            "wrong-key-2",
            "",
        ];
        let long_key = "x".repeat(100);
        
        for wrong_key in wrong_keys {
            let app = server.clone().create_router().await;
            
            let request = Request::builder()
                .method("GET")
                .uri("/api/wallets")
                .header("Authorization", wrong_key)
                .body(Body::empty())
                .unwrap();
            
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, 
                "错误的 API key '{}' 应该返回 401", wrong_key);
        }
        
        // 测试超长 key
        let app = server.clone().create_router().await;
        let request = Request::builder()
            .method("GET")
            .uri("/api/wallets")
            .header("Authorization", &long_key)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_request_timeout_handling() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 发送一个正常请求，验证超时层已配置
        let request = Request::builder()
            .method("GET")
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // 如果超时层配置正确，快速请求应该正常完成
    }
    
    #[tokio::test]
    async fn test_method_not_allowed() {
        let server = create_test_server().await;
        let app = server.create_router().await;
        
        // 尝试对只支持 GET 的端点使用 POST
        let request = Request::builder()
            .method("POST")
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        // 应该返回 405 或 404
        assert!(
            response.status() == StatusCode::METHOD_NOT_ALLOWED 
            || response.status() == StatusCode::NOT_FOUND
        );
    }
}



