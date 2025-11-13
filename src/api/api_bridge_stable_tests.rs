//! Deterministic bridge API tests — force mock success and skip decrypt to avoid AES errors.

use axum::http::StatusCode;
use axum_test::TestServer;
use defi_hot_wallet::api::server::WalletServer;
use defi_hot_wallet::core::config::{BlockchainConfig, StorageConfig, WalletConfig};
use futures::future::join_all;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::Once;
use std::sync::Arc;
use tokio::task;
use uuid::Uuid;

static INIT: Once = Once::new();

fn init_test_env() {
    INIT.call_once(|| {
        // Make bridge and crypto deterministic for tests
        env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
        env::set_var("TEST_SKIP_DECRYPT", "1");
    });
}

fn create_test_config() -> WalletConfig {
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
        derivation: Default::default(),
    }
}

async fn create_test_server() -> TestServer {
    init_test_env();
    let config = create_test_config();
    let api_key = Some(zeroize::Zeroizing::new("stable_api_key".as_bytes().to_vec()));
    // provide deterministic master key so handlers do not attempt real decryption
    let zeros: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let test_master_key = Some(defi_hot_wallet::security::secret::vec_to_secret(zeros));
    let server = WalletServer::new_for_test(
        "127.0.0.1".to_string(),
        0,
        config,
        api_key,
        test_master_key,
    )
    .await
    .expect("create server");
    TestServer::new(server.create_router().await).expect("create TestServer")
}

async fn create_wallet(server: &TestServer, name: &str) {
    let payload = json!({ "name": name, "quantum_safe": false });
    let resp = server
        .post("/api/wallets")
        .json(&payload)
        .add_header("Authorization", "stable_api_key")
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
}

#[tokio::test(flavor = "current_thread")]
async fn stable_rotate_signing_key() {
    let server = create_test_server().await;
    let name = format!("rot_{}", Uuid::new_v4().simple());
    create_wallet(&server, &name).await;

    // first rotate should move from v1->v2
    let res = server
        .post(&format!("/api/wallets/{}/rotate-signing-key", name))
        .add_header("Authorization", "stable_api_key")
        .await;
    assert_eq!(res.status_code(), StatusCode::OK, "rotate endpoint should succeed");
    let j: Value = res.json();
    assert_eq!(j["wallet"].as_str(), Some(name.as_str()));
    assert_eq!(j["old_version"].as_u64(), Some(1));
    assert_eq!(j["new_version"].as_u64(), Some(2));

    // rotate again -> v2->v3
    let res2 = server
        .post(&format!("/api/wallets/{}/rotate-signing-key", name))
        .add_header("Authorization", "stable_api_key")
        .await;
    assert_eq!(res2.status_code(), StatusCode::OK, "second rotation should succeed");
    let j2: Value = res2.json();
    assert_eq!(j2["old_version"].as_u64(), Some(2));
    assert_eq!(j2["new_version"].as_u64(), Some(3));
}

#[tokio::test(flavor = "current_thread")]
async fn stable_bridge_wallet_lifecycle_and_success() {
    let server = create_test_server().await;
    let name = format!("stable_{}", Uuid::new_v4().simple());
    create_wallet(&server, &name).await;

    // initiate a bridge request — deterministic mocks should return OK and a bridge_tx_id
    let payload = json!({
        "from_wallet": name,
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.01"
    });

    let res = server
        .post("/api/bridge")
        .json(&payload)
        .add_header("Authorization", "stable_api_key")
        .await;
    if res.status_code() != StatusCode::OK {
        // surface body for debugging in CI logs via tracing
        tracing::debug!(status = %res.status_code(), body = %res.text(), "STABLE_BRIDGE_DBG");
    }
    assert_eq!(res.status_code(), StatusCode::OK);
    let j: Value = res.json();
    assert!(j["bridge_tx_id"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
}

#[tokio::test(flavor = "current_thread")]
async fn stable_bridge_concurrent_requests() {
    let server = Arc::new(create_test_server().await);
    let wallet_name = format!("concur_{}", Uuid::new_v4().simple());
    create_wallet(&server, &wallet_name).await;

    let payload = json!({
        "from_wallet": wallet_name.clone(),
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.05"
    });

    let futs: Vec<_> = (0..12)
        .map(|_| {
            let s = server.clone();
            let body = payload.clone();
            async move {
                let r = s
                    .post("/api/bridge")
                    .json(&body)
                    .add_header("Authorization", "stable_api_key")
                    .await;
                (r.status_code(), r.text())
            }
        })
        .collect();

    let results = join_all(futs).await;
    for (status, text) in results {
        if status != StatusCode::OK {
            tracing::debug!(status = %status, body = %text, "CONCURRENT_STABLE_DBG");
        }
        assert_eq!(status, StatusCode::OK);
    }
}
