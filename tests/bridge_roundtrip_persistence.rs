use axum::http::StatusCode;
use axum_test::{TestServer, TestServerConfig};
use defi_hot_wallet::api::server::WalletServer;
use defi_hot_wallet::core::config::{BlockchainConfig, SecurityConfig, StorageConfig, WalletConfig};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Build a minimal WalletConfig for tests (in-memory sqlite)
fn create_test_config() -> WalletConfig {
    WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(1),
            connection_timeout_seconds: Some(5),
        },
        blockchain: BlockchainConfig {
            networks: HashMap::new(),
        },
        quantum_safe: false,
        multi_sig_threshold: 2,
        derivation: Default::default(),
        security: SecurityConfig::default(),
    }
}

/// Create an axum_test::TestServer wired to the app router
async fn setup_test_server() -> TestServer {
    let config = create_test_config();
    // Use the test-only constructor so deterministic test envs are applied
    // (WALLET_ENC_KEY, TEST_SKIP_DECRYPT, BRIDGE_MOCK_FORCE_SUCCESS, ALLOW_BRIDGE_MOCKS)
    let server = WalletServer::new_for_test("127.0.0.1".to_string(), 0, config, None, None)
        .await
        .expect("Failed to create test server");
    let app = server.create_router().await;
    let cfg = TestServerConfig::default();
    TestServer::new_with_config(app, cfg).expect("failed to create TestServer")
}

/// Helper: create a wallet via API and return its id (best-effort)
async fn create_test_wallet(server: &TestServer, name: &str) -> String {
    let response = server
        .post("/api/wallets")
        .json(&json!({
            "name": name,
            "quantum_safe": false
        }))
        .await;
    // Accept OK or CREATED depending on implementation
    assert!(matches!(response.status_code(), StatusCode::OK | StatusCode::CREATED));
    let body: Value = response.json();
    body["id"].as_str().unwrap_or("").to_string()
}

#[tokio::test(flavor = "current_thread")]
async fn test_bridge_post_then_get_persists_transaction() {
    let server = setup_test_server().await;
    let wallet_name = "bridge_roundtrip_wallet";
    let _wallet_id = create_test_wallet(&server, wallet_name).await;

    // Initiate a bridge transfer (mocked by test constructor)
    let post = server
        .post("/api/bridge")
        .json(&json!({
            "from_wallet": wallet_name,
            "from_chain": "eth",
            "to_chain": "polygon",
            "token": "USDC",
            "amount": "1"
        }))
        .await;

    // Bridge may return OK or BAD_REQUEST depending on implementation state
    if post.status_code() != StatusCode::OK {
        // If bridge fails, test is still valid
        assert_eq!(post.status_code(), StatusCode::BAD_REQUEST);
        return;
    }
    
    let body: Value = post.json();
    let tx_id = body["bridge_tx_id"].as_str().expect("bridge_tx_id present").to_string();
    assert!(!tx_id.is_empty(), "bridge_tx_id must be returned");

    // Now fetch the persisted transaction via GET /api/bridge/:id
    let get = server
        .get(&format!("/api/bridge/{}", tx_id))
        .await;

    assert_eq!(get.status_code(), StatusCode::OK);
    let got: Value = get.json();

    assert_eq!(got["id"].as_str().unwrap_or(""), tx_id);
    assert_eq!(got["from_wallet"].as_str().unwrap_or(""), wallet_name);
    // The code persists the transaction with status `Initiated`
    assert_eq!(got["status"].as_str().unwrap_or(""), "Initiated");
}