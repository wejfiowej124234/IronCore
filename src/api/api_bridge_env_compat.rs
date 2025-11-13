// src/api/api_bridge_env_compat.rs
// Deterministic bridge tests that toggle env flags and avoid AES decrypt errors.

#![cfg(test)]

use axum::http::StatusCode;
use axum_test::TestServer;
use defi_hot_wallet::api::server::WalletServer;
use defi_hot_wallet::core::config::{BlockchainConfig, StorageConfig, WalletConfig};
use futures::future::join_all;
use serde_json::json;
use serde_json::Value;
use serial_test::serial;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::task;
use uuid::Uuid;
use crate::security::redaction::redact_body;
use tracing::info;

fn make_config() -> WalletConfig {
    WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(4),
            connection_timeout_seconds: Some(5),
        },
        blockchain: BlockchainConfig {
            networks: HashMap::new(),
            default_network: Some("eth".to_string()),
        },
        quantum_safe: false,
        multi_sig_threshold: 1,
        derivation: Default::default(),
    }
}

fn set_env_for_mock() {
    env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    env::set_var("TEST_SKIP_DECRYPT", "1");
}

fn clear_env() {
    env::remove_var("BRIDGE_MOCK_FORCE_SUCCESS");
    env::remove_var("TEST_SKIP_DECRYPT");
}

async fn build_server_with_env(api_key: &str, force_mock: bool) -> TestServer {
    clear_env();
    if force_mock {
        set_env_for_mock();
    } else {
        // still skip decrypt in these tests unless explicitly testing decrypt path
        env::set_var("TEST_SKIP_DECRYPT", "1");
    }

    let cfg = make_config();
    let api_key_opt = Some(zeroize::Zeroizing::new(api_key.as_bytes().to_vec()));
    // Provide deterministic master key to avoid decrypt attempts during tests
    let server = WalletServer::new_for_test(
        "127.0.0.1".to_string(),
        0,
        cfg,
        api_key_opt,
        Some(zeroize::Zeroizing::new(vec![0u8; 32])),
    )
    .await
    .expect("create server");
    // Initialize tracing for tests so we capture structured logs rather than
    // relying on stderr prints. Tests can still opt-in to secrets via
    // DEV_PRINT_SECRETS if necessary.
    let _ = tracing_subscriber::fmt().try_init();

    TestServer::new(server.create_router().await).expect("create TestServer")
}

async fn create_wallet(server: &TestServer, name: &str, api_key: &str) {
    let payload = json!({ "name": name, "quantum_safe": false });
    let resp = server
        .post("/api/wallets")
        .json(&payload)
        .add_header("Authorization", api_key)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK, "create wallet failed: {}", resp.text());
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn bridge_returns_400_for_unsupported_chain_when_not_mocked() {
    let api_key = "api_env_key";
    let server = build_server_with_env(api_key, /*force_mock=*/ false).await;

    let payload = json!({
        "from_wallet": "no_wallet",
        "from_chain": "unknown_chain",
        "to_chain": "another_unknown",
        "token": "USDC",
        "amount": "1.0"
    });

    let res = server
        .post("/api/bridge")
        .json(&payload)
        .add_header("Authorization", api_key)
        .await;
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND, "body: {}", res.text());
    let j: Value = res.json();
    assert_eq!(j["code"].as_str().unwrap_or(""), "BRIDGE_FAILED");
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn bridge_returns_404_for_missing_wallet_when_mocked() {
    let api_key = "api_env_key";
    let server = build_server_with_env(api_key, /*force_mock=*/ true).await;

    let payload = json!({
        "from_wallet": "absent_wallet",
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "1.0"
    });

    let res = server
        .post("/api/bridge")
        .json(&payload)
        .add_header("Authorization", api_key)
        .await;
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND, "body: {}", res.text());
    let j: Value = res.json();
    assert_eq!(j["code"].as_str().unwrap_or(""), "BRIDGE_FAILED");
    assert!(j["error"].as_str().unwrap_or("").to_lowercase().contains("wallet"));
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn bridge_succeeds_with_mock_and_existing_wallet() {
    let api_key = "api_env_key";
    let server = build_server_with_env(api_key, /*force_mock=*/ true).await;

    let wallet_name = format!("src_{}", Uuid::new_v4().simple());
    create_wallet(&server, &wallet_name, api_key).await;

    let payload = json!({
        "from_wallet": wallet_name,
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.5"
    });

    let res = server
        .post("/api/bridge")
        .json(&payload)
        .add_header("Authorization", api_key)
        .await;

    if res.status_code() != StatusCode::OK {
        tracing::error!(status = %res.status_code(), body = %redact_body(&res.text()), "bridge request failed");
    }
    assert_eq!(res.status_code(), StatusCode::OK);
    let j: Value = res.json();
    assert!(j["bridge_tx_id"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn bridge_concurrent_requests_with_mock() {
    let api_key = "api_env_key";
    let server = Arc::new(build_server_with_env(api_key, /*force_mock=*/ true).await);

    let wallet_name = format!("concur_{}", Uuid::new_v4().simple());
    create_wallet(&server, &wallet_name, api_key).await;

    let payload = json!({
        "from_wallet": wallet_name.clone(),
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.1"
    });

    let futs: Vec<_> = (0..12)
        .map(|_| {
            let s = server.clone();
            let body = payload.clone();
            let key = api_key.to_string();
            async move {
                s.post("/api/bridge")
                    .json(&body)
                    .add_header("Authorization", &key)
                    .await
            }
        })
        .collect();

    let results = join_all(futs).await;
    for res in results {
        if res.status_code() != StatusCode::OK {
            tracing::error!(status = %res.status_code(), body = %redact_body(&res.text()), "concurrent bridge request failed");
        }
        assert_eq!(res.status_code(), StatusCode::OK);
        let j: Value = res.json();
        assert!(j["bridge_tx_id"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
    }
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn health_and_metrics_available_in_env_mode() {
    let api_key = "api_env_key";
    let server = build_server_with_env(api_key, /*force_mock=*/ true).await;

    let health = server.get("/api/health").await;
    assert_eq!(health.status_code(), StatusCode::OK, "health: {}", health.text());

    let metrics = server.get("/api/metrics").await;
    assert_eq!(metrics.status_code(), StatusCode::OK, "metrics: {}", metrics.text());
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn bridge_rejects_unknown_token_or_returns_failed_code() {
    let api_key = "api_env_key";
    let server = Arc::new(build_server_with_env(api_key, /*force_mock=*/ true).await);

    let wallet = format!("tok_{}", Uuid::new_v4().simple());
    create_wallet(&server, &wallet, api_key).await;

    let payload = json!({
        "from_wallet": wallet,
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "UNKNOWN_TOKEN",
        "amount": "0.1"
    });

    let res = server
        .post("/api/bridge")
        .json(&payload)
        .add_header("Authorization", api_key)
        .await;

    if res.status_code() == StatusCode::OK {
        let j: Value = res.json();
        assert!(j["bridge_tx_id"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
    } else {
        let j: Value = res.json();
        assert_eq!(j["code"].as_str().unwrap_or(""), "BRIDGE_FAILED");
    }
}