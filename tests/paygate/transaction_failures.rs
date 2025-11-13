//! 交易失败场景测试
//! 
//! 覆盖：余额不足、gas不足、nonce错误、网络超时等

use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_send_transaction_insufficient_balance() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "999999999999",
            "network": "eth"
        }))
        .await;
    
    // 应该返回余额不足错误
    assert!(response.status_code().is_client_error() || response.status_code().is_server_error());
}

#[tokio::test]
async fn test_send_transaction_invalid_nonce() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "1.0",
            "network": "eth",
            "nonce": 99999
        }))
        .await;
    
    // nonce过大应该失败
    assert!(response.status_code().is_client_error() || response.status_code().is_server_error());
}

#[tokio::test]
async fn test_send_transaction_gas_limit_exceeded() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "0.001",
            "network": "eth",
            "gas_limit": 1
        }))
        .await;
    
    // gas太少应该失败
    assert!(response.status_code().is_client_error() || response.status_code().is_server_error());
}

#[tokio::test]
async fn test_send_transaction_invalid_recipient_address() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "invalid_address",
            "amount": "0.001",
            "network": "eth"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_negative_amount() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "-1.0",
            "network": "eth"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_zero_amount() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "0",
            "network": "eth"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_wallet_not_found() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post("/api/v1/wallets/nonexistent_wallet/send")
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "0.001",
            "network": "eth"
        }))
        .await;
    
    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_send_transaction_missing_network() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "0.001"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_unsupported_network() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "0.001",
            "network": "unsupported_chain"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_malformed_json() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .text("{invalid json}")
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_empty_recipient() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "",
            "amount": "0.001",
            "network": "eth"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_amount_overflow() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post(&format!("/api/v1/wallets/{}/send", "test_wallet"))
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "999999999999999999999999999999",
            "network": "eth"
        }))
        .await;
    
    assert!(response.status_code().is_client_error() || response.status_code().is_server_error());
}

#[tokio::test]
async fn test_send_transaction_special_chars_in_wallet_name() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post("/api/v1/wallets/wallet@#$/send")
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "0.001",
            "network": "eth"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_send_transaction_sql_injection_attempt() {
    let server = TestServer::new(crate::api::server::create_test_router().await).unwrap();
    
    let response = server
        .post("/api/v1/wallets/'; DROP TABLE wallets; --/send")
        .add_header("X-API-Key", "test_key")
        .json(&json!({
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
            "amount": "0.001",
            "network": "eth"
        }))
        .await;
    
    assert_eq!(response.status_code(), 400);
}

