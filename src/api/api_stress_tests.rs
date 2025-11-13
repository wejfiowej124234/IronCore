//! Additional API stress/concurrency tests.
//! Uses deterministic test env flags to avoid flaky crypto/decrypt paths.

use axum::http::StatusCode;
use axum_test::TestServer;
use defi_hot_wallet::api::server::WalletServer;
use defi_hot_wallet::core::config::{BlockchainConfig, StorageConfig, WalletConfig};
use futures::future::join_all;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Once};
use tokio::task;
use uuid::Uuid;

static STRESS_INIT: Once = Once::new();

fn ensure_stress_env() {
    STRESS_INIT.call_once(|| {
        // Force mocks and skip decrypt to keep tests deterministic and fast.
        env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
        env::set_var("TEST_SKIP_DECRYPT", "1");
    });
}

fn create_test_config() -> WalletConfig {
    WalletConfig {
        storage: StorageConfig {
            // sqlite in-memory DB for isolated, fast tests
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(4),
            connection_timeout_seconds: Some(5),
        },
        blockchain: BlockchainConfig {
            networks: HashMap::new(),
            default_network: Some("eth".to_string()),
        },
        quantum_safe: false,
        multi_sig_threshold: 2,
    }
}

async fn create_test_server() -> TestServer {
    ensure_stress_env();
    let config = create_test_config();
    let api_key = Some(zeroize::Zeroizing::new("stress_api_key".as_bytes().to_vec()));
    // deterministic master key so creation/restore flows do not hit decrypt errors
    let test_master_key = Some(defi_hot_wallet::security::secret::vec_to_secret(std::iter::repeat_n(0u8, 32).collect::<Vec<u8>>()));
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

async fn create_test_wallet(server: &TestServer, name: &str) {
    let payload = json!({ "name": name, "quantum_safe": false });
    let response = server
        .post("/api/wallets")
        .json(&payload)
        .add_header("Authorization", "stress_api_key")
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test(flavor = "current_thread")]
async fn stress_test_mass_wallet_creation() {
    let server = Arc::new(create_test_server().await);

    // spawn many wallet creations concurrently
    let mut handles = Vec::new();
    for i in 0..40 {
        let s = server.clone();
        let name = format!("mass_{}", i);
        handles.push(task::spawn(async move {
            let payload = json!({ "name": name.clone(), "quantum_safe": false });
            let resp = s
                .post("/api/wallets")
                .json(&payload)
                .add_header("Authorization", "stress_api_key")
                .await;
            (resp.status_code(), name)
        }));
    }

    let mut created = 0usize;
    for h in handles {
        let (status, _name) = h.await.expect("join");
        assert_eq!(status, StatusCode::OK);
        created += 1;
    }

    // list wallets and assert at least the created count present
    let list = server
        .get("/api/wallets")
        .add_header("Authorization", "stress_api_key")
        .await;
    assert_eq!(list.status_code(), StatusCode::OK);
    let body: Value = list.json();
    let arr = body.as_array().expect("wallets array");
    assert!(arr.len() >= created, "expected at least {} wallets, got {}", created, arr.len());
}

#[tokio::test(flavor = "current_thread")]
async fn stress_test_bridge_high_concurrency() {
    let server = Arc::new(create_test_server().await);

    // create a wallet to act as from_wallet
    let wallet_name = format!("bridge_src_{}", Uuid::new_v4().simple());
    create_test_wallet(&server, &wallet_name).await;

    let payload = json!({
        "from_wallet": wallet_name.clone(),
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.1"
    });

    // fire many concurrent bridge requests and ensure all return OK and a bridge_tx_id
    let futs: Vec<_> = (0..24)
        .map(|_| {
            let s = server.clone();
            let body = payload.clone();
            async move {
                s.post("/api/bridge")
                    .json(&body)
                    .add_header("Authorization", "stress_api_key")
                    .await
            }
        })
        .collect();

    let results = join_all(futs).await;
    for res in results {
        if res.status_code() != StatusCode::OK {
            eprintln!("BRIDGE_CONC_DBG: {} body: {}", res.status_code(), res.text());
        }
        assert_eq!(res.status_code(), StatusCode::OK);
        let j: Value = res.json();
        assert!(j["bridge_tx_id"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn stress_test_backup_delete_restore_cycle() {
    let server = create_test_server().await;

    let name = format!("cycle_{}", Uuid::new_v4().simple());
    create_test_wallet(&server, &name).await;

    // backup -> get seed
    let b = server
        .get(&format!("/api/wallets/{}/backup", name))
        .add_header("Authorization", "stress_api_key")
        .await;
    assert_eq!(b.status_code(), StatusCode::OK);
    let body: Value = b.json();
    assert_eq!(body["alg"], "PLAINTEXT");
    let seed = body["ciphertext"].as_str().unwrap_or("").to_string();
    assert!(!seed.is_empty());

    // delete wallet
    let del = server
        .delete(&format!("/api/wallets/{}", name))
        .add_header("Authorization", "stress_api_key")
        .await;
    assert!(matches!(del.status_code(), StatusCode::OK | StatusCode::NO_CONTENT));

    // restore into a different name
    let new_name = format!("restored_{}", Uuid::new_v4().simple());
    let payload = json!({ "name": new_name.clone(), "seed_phrase": seed, "quantum_safe": false });
    let r = server
        .post("/api/wallets/restore")
        .json(&payload)
        .add_header("Authorization", "stress_api_key")
        .await;
    assert_eq!(r.status_code(), StatusCode::OK);
    let restored: Value = r.json();
    assert_eq!(restored["name"], new_name);
}

#[tokio::test(flavor = "current_thread")]
async fn stress_test_auth_header_edge_cases() {
    let server = create_test_server().await;

    // completely missing header -> UNAUTHORIZED
    let r = server.get("/api/wallets").await;
    assert_eq!(r.status_code(), StatusCode::UNAUTHORIZED);

    // empty header value -> UNAUTHORIZED
    let r2 = server.get("/api/wallets").add_header("Authorization", "").await;
    assert_eq!(r2.status_code(), StatusCode::UNAUTHORIZED);

    // whitespace-only -> UNAUTHORIZED
    let r3 = server.get("/api/wallets").add_header("Authorization", "   ").await;
    assert_eq!(r3.status_code(), StatusCode::UNAUTHORIZED);

    // valid key works
    let r4 = server.get("/api/wallets").add_header("Authorization", "stress_api_key").await;
    assert_eq!(r4.status_code(), StatusCode::OK);
}