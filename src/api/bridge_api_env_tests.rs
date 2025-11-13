// tests/bridge_api_env_tests.rs
// Deterministic API tests for bridge flows that force mock success / skip decrypt.

use axum::http::StatusCode;
use axum_test::TestServer;
use defi_hot_wallet::api::server::WalletServer;
use defi_hot_wallet::core::config::{BlockchainConfig, StorageConfig, WalletConfig};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use futures::future::join_all;
use tokio::task;
use uuid::Uuid;

fn make_test_config() -> WalletConfig {
    WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(2),
            connection_timeout_seconds: Some(5),
        },
        blockchain: BlockchainConfig {
            networks: HashMap::new(),
            default_network: Some("eth".to_string()),
        },
        quantum_safe: false,
        multi_sig_threshold: 1,
    }
}

async fn build_test_server() -> TestServer {
    // Ensure deterministic mock and skip decrypt for these tests.
    env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    env::set_var("TEST_SKIP_DECRYPT", "1");

    let cfg = make_test_config();
    let api_key = Some(zeroize::Zeroizing::new("env_test_key".as_bytes().to_vec()));
    // Provide a test master key to avoid decrypt attempts in handlers
    let srv = WalletServer::new_for_test(
        "127.0.0.1".to_string(),
        0,
        cfg,
        api_key.clone(),
    Some(defi_hot_wallet::security::secret::vec_to_secret(std::iter::repeat_n(0u8, 32).collect::<Vec<u8>>())),
    )
        .await
        .expect("create WalletServer for test");
    TestServer::new(srv.create_router().await).expect("create TestServer")
}

async fn create_wallet(server: &TestServer, name: &str) {
    let payload = json!({ "name": name, "quantum_safe": false });
    let resp = server
        .post("/api/wallets")
        .json(&payload)
        .add_header("Authorization", "env_test_key")
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
}

#[tokio::test(flavor = "current_thread")]
async fn test_bridge_assets_succeeds_with_env_flags() {
    let server = build_test_server().await;
    let wallet = format!("bridge_env_{}", Uuid::new_v4().simple());
    create_wallet(&server, &wallet).await;

    let payload = json!({
        "from_wallet": wallet,
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.01"
    });

    let res = server
        .post("/api/bridge")
        .json(&payload)
        .add_header("Authorization", "env_test_key")
        .await;

    if res.status_code() != StatusCode::OK {
        eprintln!("DBG BODY: {}", res.text());
    }
    assert_eq!(res.status_code(), StatusCode::OK);
    let j: Value = res.json();
    assert!(j["bridge_tx_id"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
}

#[tokio::test(flavor = "current_thread")]
async fn test_bridge_concurrent_requests_with_env_flags() {
    let server = Arc::new(build_test_server().await);
    let wallet = format!("bridge_concur_{}", Uuid::new_v4().simple());
    create_wallet(&server, &wallet).await;

    let payload = json!({
        "from_wallet": wallet.clone(),
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.02"
    });

    let futs: Vec<_> = (0..12)
        .map(|_| {
            let s = server.clone();
            let body = payload.clone();
            async move {
                let r = s
                    .post("/api/bridge")
                    .json(&body)
                    .add_header("Authorization", "env_test_key")
                    .await;
                (r.status_code(), r.text())
            }
        })
        .collect();

    let results = join_all(futs).await;
    for (status, body) in results {
        if status != StatusCode::OK {
            eprintln!("CONCURRENT DBG: {} body: {}", status, body);
        }
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_backup_delete_restore_cycle_with_env_flags() {
    let server = build_test_server().await;
    let name = format!("cycle_env_{}", Uuid::new_v4().simple());
    create_wallet(&server, &name).await;

    // backup
    let b = server
        .get(&format!("/api/wallets/{}/backup", name))
        .add_header("Authorization", "env_test_key")
        .await;
    assert_eq!(b.status_code(), StatusCode::OK);
    let body: Value = b.json();
    // test-env returns plaintext wrapped in ciphertext with alg==PLAINTEXT
    assert_eq!(body["alg"], "PLAINTEXT");
    let seed = body["ciphertext"].as_str().unwrap_or("").to_string();
    assert!(!seed.is_empty());

    // delete
    let del = server
        .delete(&format!("/api/wallets/{}", name))
        .add_header("Authorization", "env_test_key")
        .await;
    assert!(matches!(del.status_code(), StatusCode::OK | StatusCode::NO_CONTENT));

    // restore into a new name
    let new_name = format!("restored_env_{}", Uuid::new_v4().simple());
    let payload = json!({ "name": new_name.clone(), "seed_phrase": seed, "quantum_safe": false });
    let r = server
        .post("/api/wallets/restore")
        .json(&payload)
        .add_header("Authorization", "env_test_key")
        .await;
    assert_eq!(r.status_code(), StatusCode::OK);
    let restored: Value = r.json();
    assert_eq!(restored["name"], new_name);
}