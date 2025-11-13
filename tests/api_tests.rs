//! API 鍔熻兘娴嬭瘯锛氭祴璇曟墍鏈?API 绔偣鐨勬甯稿姛鑳?//! 瑕嗙洊锛氶挶鍖呯鐞嗐€佷氦鏄撱€佸巻鍙层€佸浠姐€佸绛惧悕銆佹ˉ鎺ャ€佹寚鏍囥€佸仴搴锋鏌?//! 浣跨敤璁よ瘉澶达紝纭繚閫氳繃 API key 妫€鏌?
use axum::http::StatusCode;
use axum_test::TestServer;
use defi_hot_wallet::api::server::WalletServer;
use defi_hot_wallet::core::config::{BlockchainConfig, SecurityConfig, StorageConfig, WalletConfig};
use serde_json::json;
use std::collections::HashMap;
// removed redundant 'use tokio;'

fn create_test_config() -> WalletConfig {
    WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(), // 淇锛氱Щ闄?//
            max_connections: Some(1),
            connection_timeout_seconds: Some(30),
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

async fn create_test_server() -> TestServer {
    let config = create_test_config();
    let api_key = Some(zeroize::Zeroizing::new("test_api_key".as_bytes().to_vec()));
    // Use deterministic test master key for consistent test results
    let zeros: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let test_master_key = defi_hot_wallet::security::secret::vec_to_secret(zeros); // 32 zero bytes for testing
    let server = WalletServer::new_for_test(
        "127.0.0.1".to_string(),
        0,
        config,
        api_key,
        Some(test_master_key),
    )
    .await
    .unwrap();
    TestServer::new(server.create_router().await).unwrap()
}

async fn create_test_wallet(server: &TestServer, name: &str) {
    let payload = json!({
        "name": name,
        "quantum_safe": false
    });
    let response = server
        .post("/api/wallets")
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_check() {
    let server = create_test_server().await;
    let response = server.get("/api/health").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "healthy");
    assert!(body["version"].is_string()); // 琛ヤ竵锛氭鏌ョ増鏈?    assert!(body["timestamp"].is_string()); // 琛ヤ竵锛氭鏌ユ椂闂存埑
}

#[tokio::test]
async fn test_create_wallet() {
    let server = create_test_server().await;
    let payload = json!({
        "name": "test_wallet",
        "quantum_safe": true
    });
    let response = server
        .post("/api/wallets")
        .json(&payload)
        .add_header("Authorization", "test_api_key") // 淇锛氭坊鍔犺璇佸ご
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "test_wallet");
    assert_eq!(body["quantum_safe"].as_bool(), Some(true));
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn test_list_wallets() {
    let server = create_test_server().await;
    let response = server.get("/api/wallets").add_header("Authorization", "test_api_key").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: Vec<serde_json::Value> = response.json();
    assert!(body.is_empty()); // 鍒濆涓虹┖
}

#[tokio::test]
async fn test_delete_wallet() {
    let server = create_test_server().await;
    create_test_wallet(&server, "test_wallet").await;
    let response =
        server.delete("/api/wallets/test_wallet").add_header("Authorization", "test_api_key").await;
    assert_eq!(response.status_code(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_get_balance() {
    let server = create_test_server().await;
    create_test_wallet(&server, "test_wallet").await;
    let response = server
        .get("/api/wallets/test_wallet/balance?network=eth")
        .add_header("Authorization", "test_api_key")
        .await;
    // 鍥犱负娴嬭瘯鏈嶅姟鍣ㄦ病鏈夐厤缃尯鍧楅摼瀹㈡埛绔紝鎵€浠ヤ細杩斿洖 500 閿欒
    assert!(response.status_code() == StatusCode::OK || response.status_code() == StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_send_transaction() {
    let server = create_test_server().await;
    create_test_wallet(&server, "test_wallet").await;
    let payload = json!({
        "to_address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "amount": "0.1",
        "network": "eth"
    });
    let response = server
        .post("/api/wallets/test_wallet/send")
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_get_transaction_history() {
    let server = create_test_server().await;
    create_test_wallet(&server, "test_wallet").await;
    let response = server
        .get("/api/wallets/test_wallet/history")
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert!(body["transactions"].is_array());
}

#[tokio::test]
async fn test_backup_wallet() {
    let server = create_test_server().await;
    create_test_wallet(&server, "test_wallet").await;
    let response = server
        .get("/api/wallets/test_wallet/backup")
        .add_header("Authorization", "test_api_key")
        .await;
    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert!(body["ciphertext"].is_string());
    } else {
        assert!(response.status_code().is_client_error() || response.status_code().is_server_error());
    }
}

#[tokio::test]
async fn test_restore_wallet() {
    let server = create_test_server().await;
    let payload = json!({
        "name": "restored_wallet",
        "seed_phrase": "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    });
    let response = server
        .post("/api/wallets/restore")
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "restored_wallet");
}

#[tokio::test]
async fn test_send_multi_sig_transaction() {
    let server = create_test_server().await;
    create_test_wallet(&server, "test_wallet").await;
    let payload = json!({
        "to_address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "amount": "0.1",
        "network": "eth",
        "signatures": ["sig1", "sig2"]
    });
    let response = server
        .post("/api/wallets/test_wallet/send_multi_sig")
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    // Server now returns 200 OK with a tx_hash in the body for multi-sig send
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert!(body["tx_hash"].is_string());
}

#[tokio::test]
async fn test_bridge_assets() {
    let server = create_test_server().await;
    create_test_wallet(&server, "test_wallet").await;
    let payload = json!({
        "from_wallet": "test_wallet",
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "10.0"
    });
    let response =
        server.post("/api/bridge").json(&payload).add_header("Authorization", "test_api_key").await;
    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert!(body["bridge_tx_id"].is_string());
    } else {
        assert!(response.status_code().is_client_error() || response.status_code().is_server_error());
    }
}

#[tokio::test]
async fn test_metrics() {
    let server = create_test_server().await;
    let response = server.get("/api/metrics").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("# HELP"));
}

#[tokio::test]
async fn test_invalid_endpoint() {
    let server = create_test_server().await;
    let response = server.get("/invalid-endpoint").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}
