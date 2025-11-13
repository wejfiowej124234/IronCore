//! Extra API tests to increase coverage and exercise edge cases.
//! These follow repository patterns: tokio current_thread, deterministic env flags.

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
use uuid::Uuid;

static ADDL_TEST_INIT: Once = Once::new();

fn ensure_addl_test_env() {
    ADDL_TEST_INIT.call_once(|| {
        env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    });
}

fn create_test_config() -> WalletConfig {
    WalletConfig {
        storage: StorageConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: Some(1),
            connection_timeout_seconds: Some(30),
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
    ensure_addl_test_env();
    let config = create_test_config();
    let api_key = Some(zeroize::Zeroizing::new("test_api_key".as_bytes().to_vec()));
    // Provide a deterministic test master key so load/decrypt works in tests
    let test_master_key = Some(defi_hot_wallet::security::secret::vec_to_secret(std::iter::repeat_n(0u8, 32).collect::<Vec<u8>>()));
    let server = WalletServer::new_for_test(
        "127.0.0.1".to_string(),
        0,
        config,
        api_key,
        test_master_key,
    )
    .await
    .unwrap();
    TestServer::new(server.create_router().await).unwrap()
}

async fn create_test_wallet(server: &TestServer, name: &str) {
    let payload = json!({ "name": name, "quantum_safe": false });
    let response = server
        .post("/api/wallets")
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test(flavor = "current_thread")]
async fn test_restore_with_invalid_seed_returns_bad_request() {
    let server = create_test_server().await;
    let payload = json!({
        "name": "bad_restore",
        "seed_phrase": "this is not a valid seed phrase"
    });
    let response = server
        .post("/api/wallets/restore")
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    assert!(body["error"].as_str().unwrap().contains("Invalid seed phrase"));
}

#[tokio::test(flavor = "current_thread")]
async fn test_backup_then_restore_cycle_using_seed() {
    let server = create_test_server().await;
    let name = format!("cycle_{}", Uuid::new_v4().simple());
    create_test_wallet(&server, &name).await;

    // backup
    let b = server
        .get(&format!("/api/wallets/{}/backup", name))
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(b.status_code(), StatusCode::OK);
    let body: Value = b.json();
    assert_eq!(body["alg"], "PLAINTEXT");
    let seed = body["ciphertext"].as_str().unwrap_or("").to_string();
    assert!(!seed.is_empty(), "seed_phrase must be present");

    // restore to new name
    let new_name = format!("restored_{}", Uuid::new_v4().simple());
    let payload = json!({ "name": new_name.clone(), "seed_phrase": seed, "quantum_safe": false });
    let r = server
        .post("/api/wallets/restore")
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    assert_eq!(r.status_code(), StatusCode::OK);
    let restored: Value = r.json();
    assert_eq!(restored["name"], new_name);
}

#[tokio::test(flavor = "current_thread")]
async fn test_multi_sig_success_returns_tx_hash() {
    let server = create_test_server().await;
    let name = format!("ms_ok_{}", Uuid::new_v4().simple());
    create_test_wallet(&server, &name).await;

    // Provide sufficient signatures (threshold=2 in test config)
    let payload = json!({
        "to_address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "amount": "0.01",
        "network": "eth",
        "signatures": ["sigA", "sigB"]
    });
    let res = server
        .post(&format!("/api/wallets/{}/send_multi_sig", name))
        .json(&payload)
        .add_header("Authorization", "test_api_key")
        .await;
    // Expect OK (mocked path) and a tx_hash string
    assert_eq!(res.status_code(), StatusCode::OK);
    let j: Value = res.json();
    assert!(j["tx_hash"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
}

#[tokio::test(flavor = "current_thread")]
async fn test_bridge_concurrent_requests_stable_under_force_success() {
    let server = Arc::new(create_test_server().await);
    let wallet_name = format!("concur_{}", Uuid::new_v4().simple());
    create_test_wallet(&server, &wallet_name).await;

    let payload = json!({
        "from_wallet": wallet_name.clone(),
        "from_chain": "eth",
        "to_chain": "polygon",
        "token": "USDC",
        "amount": "0.5"
    });

    let futs: Vec<_> = (0..8)
        .map(|_| {
            let s = server.clone();
            let body = payload.clone();
            async move {
                s.post("/api/bridge")
                    .json(&body)
                    .add_header("Authorization", "test_api_key")
                    .await
            }
        })
        .collect();

    let results = join_all(futs).await;
    for res in results {
        if res.status_code() != StatusCode::OK {
            eprintln!("CONCURRENT DEBUG: {} body: {}", res.status_code(), res.text());
        }
        assert_eq!(res.status_code(), StatusCode::OK);
        let j: Value = res.json();
        assert!(j["bridge_tx_id"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_auth_header_variants_and_missing_header() {
    let server = create_test_server().await;

    // missing header -> UNAUTHORIZED
    let r = server.get("/api/wallets").await;
    assert_eq!(r.status_code(), StatusCode::UNAUTHORIZED);

    // wrong scheme (Bearer) -> UNAUTHORIZED
    let r2 = server.get("/api/wallets").add_header("Authorization", "Bearer test_api_key").await;
    assert_eq!(r2.status_code(), StatusCode::UNAUTHORIZED);

    // correct bare key -> OK
    let r3 = server.get("/api/wallets").add_header("Authorization", "test_api_key").await;
    assert_eq!(r3.status_code(), StatusCode::OK);
}