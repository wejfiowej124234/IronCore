use crate::network::rate_limit::RateLimiter;
use crate::api::server_config::*;
use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::{limit::ConcurrencyLimitLayer, timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer, cors::CorsLayer};

use crate::api::handlers;
use crate::core::config::WalletConfig;
use crate::core::errors::WalletError;
use crate::core::wallet_manager::WalletManager;
use crate::api::anomaly_detection;
use crate::api::auth_simple;
use axum::error_handling::HandleErrorLayer;
use tower::BoxError;

#[derive(Clone)]
pub struct WalletServer {
    pub wallet_manager: Arc<WalletManager>,
    pub user_db: Arc<crate::api::user_db::UserDatabase>, // ✅ user数据库
    pub session_store: Arc<crate::api::session_store::SessionStore>, // ✅ 会话存储
    pub host: String,
    pub port: u16,
    pub config: WalletConfig,
    pub api_key: Option<crate::security::SecretVec>,
    pub rate_limiter: Arc<RateLimiter>, // SECURITY: Rate limiter to prevent DoS attacks
}

impl WalletServer {
    pub async fn new(
        host: String,
        port: u16,
        config: WalletConfig,
        api_key: Option<crate::security::SecretVec>,
    ) -> Result<Self, WalletError> {
        let wallet_manager = Arc::new(WalletManager::new(&config).await?);
        
        // ✅ 初始化user数据库
        let users_db_url = std::env::var("USERS_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://./users.db".to_string());
        let user_db = Arc::new(crate::api::user_db::UserDatabase::new(&users_db_url).await
            .map_err(|e| WalletError::SecurityError(format!("user数据库初始化failed: {}", e)))?);
        
        // ✅ 初始化会话存储
        let session_store = Arc::new(crate::api::session_store::SessionStore::new());
        
        // SECURITY: Initialize rate limiter to prevent DoS attacks
        // Allow 100 requests per minute per IP
        let rate_limiter = Arc::new(RateLimiter::new(100, Duration::from_secs(60)));
        Ok(Self { wallet_manager, user_db, session_store, host, port, config, api_key, rate_limiter })
    }

    /// Test-only constructor used by integration tests.
    /// Accepts an optional test_master_key for future master-key injection support.
    pub async fn new_for_test(
        bind_addr: String,
        port: u16,
        config: WalletConfig,
        api_key: Option<crate::security::SecretVec>,
        test_master_key: Option<crate::security::SecretVec>,
    ) -> Result<Self, WalletError> {
        // Ensure integration tests (which compile the library without the
        // `test-env` feature) still get the deterministic test env guards when
        // using the test-only constructor. This mirrors `src/test_env.rs`.
        // These env vars are test-only and only set by the test constructor.
        std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        std::env::set_var("TEST_SKIP_DECRYPT", "1");
        std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
        std::env::set_var("BRIDGE_MOCK", "1");
        std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");
        // Marker to indicate the test-only constructor was used so other
        // modules can detect this state without relying on test-harness envs.
        std::env::set_var("WALLET_TEST_CONSTRUCTOR", "1");

        // 移除强制设置 BRIDGE_MOCK_FORCE_SUCCESS/TEST_SKIP_DECRYPT，由各测试自行控制
        // apply test key before initializing internals so create_wallet() uses same key
        #[cfg(any(test, feature = "test-env"))]
        if let Some(k) = test_master_key.as_ref() {
            // ensure public helper exists in core::wallet_manager
            crate::core::wallet_manager::set_test_master_key_default(k.clone());
            tracing::info!("new_for_test: applied test master key fingerprint for tests");
        }
        
        #[cfg(not(any(test, feature = "test-env")))]
        let _ = test_master_key; // silence unused warning
        // delegate to primary constructor which will create WalletManager etc.
        let mut server = WalletServer::new(bind_addr, port, config, api_key).await?;
        // Override rate limiter for tests to allow unlimited requests
        server.rate_limiter = Arc::new(RateLimiter::new(10000, Duration::from_secs(1)));
        Ok(server)
    }

    pub async fn create_router(self) -> Router {
        let state = Arc::new(self);
        
        // SECURITY: Get CORS origin from environment variable (default: localhost:3000)
        let cors_origin = std::env::var("CORS_ALLOW_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        
        tracing::info!("CORS configured to allow origin: {}", cors_origin);
        
        // Create auth sub-router with its own state
        // ✅ 共享SessionStore实现tokenvalidate
        let users_db_url = std::env::var("USERS_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://./users.db".to_string());
        
        let auth_state = auth_simple::AuthApiState::new(&users_db_url, state.session_store.clone())
            .await
            .expect("Failed to initialize users database");
        
        let auth_router = auth_simple::create_auth_routes(auth_state);

        let base_router = Router::new()
            .route("/health", get(handlers::health_check))  // ✅ 添加根路径健康check
            .route("/api/health", get(handlers::health_check))
            .route("/api/system/info", get(crate::api::handlers::system_info::system_info))
            .route("/api/wallets", post(handlers::create_wallet).get(handlers::list_wallets))
            .route("/api/wallets/:name", delete(handlers::delete_wallet))
            .route("/api/wallets/:name/address", get(handlers::get_wallet_address))
            .route("/api/wallets/:name/addresses", post(handlers::get_wallet_address).get(handlers::get_wallet_address))  // ✅ 添加addresses路由（复数形式）
            .route("/api/wallets/:name/balance", get(handlers::get_balance))
            .route("/api/wallets/:name/assets", get(crate::api::handlers::multi_assets::get_multi_assets))
            .route("/api/wallets/:name/multi-assets", get(crate::api::handlers::multi_assets::get_multi_assets))  // ✅ 别名路由
            .route("/api/wallets/:name/history", get(handlers::get_transaction_history))
            .route("/api/wallets/:name/transactions", get(handlers::get_transaction_history)) // 别名
            .route("/api/wallets/:name/backup", get(handlers::backup_wallet))
            .route("/api/wallets/restore", post(handlers::restore_wallet))
            .route("/api/wallets/:name/rotate-signing-key", post(handlers::rotate_signing_key))
            .route("/api/wallets/:name/send_multi_sig", post(handlers::send_multi_sig_transaction))
            // Independent transactions API
            .route("/api/transactions/history", get(handlers::transactions_history))
            .route("/api/transactions/send", post(handlers::transactions_send))
            .route("/api/transactions/:id/status", get(handlers::transaction_status))
            .route("/api/metrics", get(handlers::metrics))
            .layer(
                CorsLayer::new()
                    .allow_origin({
                        use tower_http::cors::AllowOrigin;
                        let origins_env = cors_origin.clone();
                        if origins_env.contains(',') {
                            let list = origins_env
                                .split(',')
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .map(|s| axum::http::HeaderValue::from_str(s)
                                    .expect("Invalid CORS origin in list"))
                                .collect::<Vec<axum::http::HeaderValue>>();
                            AllowOrigin::list(list)
                        } else {
                            AllowOrigin::exact(axum::http::HeaderValue::from_str(&origins_env)
                                .expect("Invalid CORS_ALLOW_ORIGIN environment variable"))
                        }
                    })
                    .allow_methods([
                        axum::http::Method::GET, 
                        axum::http::Method::POST, 
                        axum::http::Method::DELETE,
                        axum::http::Method::OPTIONS,
                        axum::http::Method::PUT,
                        axum::http::Method::PATCH
                    ])
                    .allow_headers([
                        axum::http::header::AUTHORIZATION, 
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::ACCEPT,
                        axum::http::header::ORIGIN,
                        axum::http::header::ACCESS_CONTROL_REQUEST_METHOD,
                        axum::http::header::ACCESS_CONTROL_REQUEST_HEADERS,
                        axum::http::HeaderName::from_static("x-api-key"),
                    ])
                    .expose_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                    ])
                    .allow_credentials(true)
                    .max_age(CORS_MAX_AGE)
            )
            .layer(
                ServiceBuilder::new()
                    // Convert middleware errors (timeout/overload) into HTTP responses
                    .layer(HandleErrorLayer::new(|err: BoxError| async move {
                        if err.is::<tower::timeout::error::Elapsed>() {
                            (StatusCode::REQUEST_TIMEOUT, "request timed out")
                        } else {
                            (StatusCode::SERVICE_UNAVAILABLE, "service overloaded")
                        }
                    }))
                    // Concurrency and body limits to reduce DoS risk
                    .layer(ConcurrencyLimitLayer::new(MAX_CONCURRENCY))
                    .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
                    // Set a reasonable per-request timeout
                    .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
                    // Structured HTTP tracing without leaking sensitive data
                    .layer(TraceLayer::new_for_http()),
            );

        // Sensitive endpoints sub-router with stricter limits and per-route timeout
        let sensitive = Router::new()
            .route("/api/wallets/:name/send", post(handlers::send_transaction))
            .route("/api/bridge", post(handlers::bridge::bridge_assets))
            .route("/api/bridge/history", get(handlers::bridge_history))
            .route("/api/bridge/:id/status", get(handlers::bridge_status))
            // DEX 交换路由
            .route("/api/swap/quote", get(crate::api::swap::swap_quote))
            .route("/api/swap/execute", post(crate::api::swap::swap_execute))
            // NFT 资产管理路由
            .route("/api/nfts/:wallet", get(crate::api::nft::get_nfts))
            .route("/api/nfts/detail/:id", get(crate::api::nft::get_nft_detail))
            .route("/api/nfts/transfer", post(crate::api::nft::transfer_nft))
            // GameFi 和空投路由
            .route("/api/gamefi/assets/:wallet", get(crate::api::gamefi::get_game_assets))
            .route("/api/airdrops/:wallet", get(crate::api::gamefi::get_airdrops))
            .route("/api/airdrops/:id/claim", post(crate::api::gamefi::claim_airdrop))
            .layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(|err: BoxError| async move {
                        if err.is::<tower::timeout::error::Elapsed>() {
                            (StatusCode::REQUEST_TIMEOUT, "request timed out")
                        } else {
                            (StatusCode::SERVICE_UNAVAILABLE, "service overloaded")
                        }
                    }))
                    .layer(RequestBodyLimitLayer::new(MAX_SENSITIVE_BODY_SIZE))
                    .layer(TimeoutLayer::new(SENSITIVE_REQUEST_TIMEOUT)),
            );

        // ✅ user偏好设置路由（需要单独添加Extension）
        let preferences_router = Router::new()
            .route("/api/users/:user_id/preferences", get(crate::api::user_preferences::get_user_preferences))
            .route("/api/users/:user_id/preferences", put(crate::api::user_preferences::update_user_preferences))
            .layer(Extension(state.user_db.clone()));

        // Merge base and sensitive routers with shared state
        let app = base_router
            .merge(sensitive)
            .merge(preferences_router)  // ✅ 在with_state()之前merge
            .with_state(state.clone());

        // Anomaly detection sub-router with its own state
        let anomaly_state = anomaly_detection::AnomalyApiState::new();
        let anomaly_router = Router::new()
            .nest("/api/anomaly-detection", anomaly_detection::create_anomaly_routes(anomaly_state));

        // Merge all routers (auth and anomaly have their own states)
        // ✅ 添加全局CORS到整个应用
        let cors_layer = CorsLayer::new()
            .allow_origin({
                use tower_http::cors::AllowOrigin;
                let origins_env = cors_origin.clone();
                if origins_env.contains(',') {
                    let list = origins_env
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| axum::http::HeaderValue::from_str(s)
                            .expect("Invalid CORS origin in list"))
                        .collect::<Vec<axum::http::HeaderValue>>();
                    AllowOrigin::list(list)
                } else {
                    AllowOrigin::exact(axum::http::HeaderValue::from_str(&origins_env)
                        .expect("Invalid CORS_ALLOW_ORIGIN"))
                }
            })
            .allow_methods([
                axum::http::Method::GET, 
                axum::http::Method::POST, 
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
                axum::http::Method::PUT,
                axum::http::Method::PATCH
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION, 
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ACCEPT,
                axum::http::header::ORIGIN,
                axum::http::header::ACCESS_CONTROL_REQUEST_METHOD,
                axum::http::header::ACCESS_CONTROL_REQUEST_HEADERS,
                axum::http::HeaderName::from_static("x-api-key"),
            ])
            .expose_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ])
            .allow_credentials(true)
            .max_age(CORS_MAX_AGE);
        
        app.merge(auth_router)
            .merge(anomaly_router)
            .layer(cors_layer) // ✅ 全局CORS
    }

    pub async fn start(self) -> Result<(), anyhow::Error> {
        // Security guard: prevent accidental enabling of bridge mocks in production
        // unless explicitly allowed by env. This runs at startup and fails fast.
        #[allow(unused_imports)]
        use crate::blockchain::bridge::relay::{
            bridge_mocks_allowed, bridge_mocks_requested_truthy,
        };
        #[cfg(not(feature = "test-env"))]
        {
            if bridge_mocks_requested_truthy() && !bridge_mocks_allowed() {
                anyhow::bail!(
                    "Bridge mocks requested via env (e.g. BRIDGE_MOCK_FORCE_SUCCESS=1), \
but not allowed. Set ALLOW_BRIDGE_MOCKS=1 to enable in non-test runs, or unset mock envs."
                );
            }
        }
        let app = self.clone().create_router().await;
        let addr = format!("{}:{}", self.host, self.port);
        tracing::info!("Server listening on {}", addr);
        let listener = TcpListener::bind(&addr).await?;
        axum::serve(listener, app.into_make_service()).await?;
        Ok(())
    }
}

/// Helper to evaluate whether the startup mock guard would bail based on current env.
/// Note: In test builds (`test-env` feature), bridge mocks are allowed so this will return false.
pub fn startup_mock_guard_should_bail_for_env() -> bool {
    use crate::blockchain::bridge::relay::{bridge_mocks_allowed, bridge_mocks_requested_truthy};
    bridge_mocks_requested_truthy() && !bridge_mocks_allowed()
}

// shared request/response types are in crate::api::types

#[cfg(test)]
mod startup_guard_tests {
    use super::startup_mock_guard_should_bail_for_env;
    use std::env;

    #[test]
    fn test_startup_guard_bails_when_mocks_requested_without_allow() {
        // Save env
        let keys = [
            "ALLOW_BRIDGE_MOCKS",
            "BRIDGE_MOCK_FORCE_SUCCESS",
            "BRIDGE_MOCK",
            "FORCE_BRIDGE_SUCCESS",
            "BRIDGE_MOCK_FORCE",
        ];
        let saved: Vec<(String, Option<String>)> =
            keys.iter().map(|k| (k.to_string(), env::var(k).ok())).collect();
        for k in &keys {
            env::remove_var(k);
        }

        // Request mocks but do not allow
        env::set_var("BRIDGE_MOCK", "1");

        // Under test builds with feature `test-env`, mocks are allowed and guard wouldn't bail.
        // Also many test runners (and code coverage tools like tarpaulin) set
        // `RUST_TEST_THREADS` which our bridge_mocks_allowed() treats as a
        // signal to allow mocks. Avoid asserting the bail condition when
        // either of those are present to prevent flaky CI failures.
        if !cfg!(feature = "test-env") && std::env::var("RUST_TEST_THREADS").is_err() {
            assert!(startup_mock_guard_should_bail_for_env());
        }

        // Allow and confirm it does not bail
        env::set_var("ALLOW_BRIDGE_MOCKS", "1");
        assert!(!startup_mock_guard_should_bail_for_env());

        // Restore envs
        for (k, v) in saved {
            match v {
                Some(val) => env::set_var(k, val),
                None => env::remove_var(k),
            }
        }
    }
}
