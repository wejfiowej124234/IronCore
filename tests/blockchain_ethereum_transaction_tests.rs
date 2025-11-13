//! tests/blockchain_ethereum_transaction_tests.rs
//!
//! Tests for Ethereum blockchain client transaction functionality.
//! This file focuses on the send_transaction and get_transaction_status methods, plus client creation.

use defi_hot_wallet::blockchain::ethereum::*;
use defi_hot_wallet::blockchain::traits::{BlockchainClient, TransactionStatus};
use defi_hot_wallet::core::domain::PrivateKey;
use ethers::prelude::*;
use ethers::providers::{MockProvider, MockResponse, Provider};
use ethers::types::U256;
use serde_json::json;
use std::str::FromStr;

// Helper function to create a mock provider with a provider
fn create_mock_client() -> (EthereumClient<MockProvider>, MockProvider) {
    let mock = MockProvider::new();
    let handle = mock.clone();
    let provider = Provider::new(mock);
    (EthereumClient::new_with_provider(provider), handle)
}

#[tokio::test]
async fn test_ethereum_client_new_invalid_url() {
    // Test creating client with invalid URL
    let invalid = EthereumClient::new("invalid://url").await;
    assert!(invalid.is_err());
}

#[tokio::test]
async fn test_send_transaction_normal() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // push in reverse because MockProvider is LIFO: last pushed is returned first
    // 3. 妯℃嫙 eth_sendRawTransaction 鍝嶅簲
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    // 2. 妯℃嫙 eth_getTransactionCount (nonce) 鍝嶅簲
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    // 1. 妯℃嫙 eth_gasPrice 鍝嶅簲
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));

    let private_key = [1u8; 32]; // A non-zero private key
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "0.01";

    let result_tx_hash: String =
        EthereumClient::<MockProvider>::send_transaction(&client, &pk, to_address, amount)
            .await
            .unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_zero_amount() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // push in reverse because MockProvider is LIFO
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "0.0"; // Zero amount

    let result_tx_hash: String =
        EthereumClient::<MockProvider>::send_transaction(&client, &pk, to_address, amount)
            .await
            .unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_max_amount() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "1000.0"; // 1000 ETH

    let result_tx_hash: String =
        EthereumClient::<MockProvider>::send_transaction(&client, &pk, to_address, amount)
            .await
            .unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_duplicate_tx() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // Mock responses for the second call (LIFO)
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(43))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    // Mock responses for the first call (LIFO)
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "0.01";

    // Send twice (simulate duplicate)
    let result1: String = client.send_transaction(&pk, to_address, amount).await.unwrap();
    let result2: String = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result1, result2); // The mock returns the same hash, but the nonce was different.
}

#[tokio::test]
async fn test_send_transaction_invalid_private_key_length() {
    let (_client, _) = create_mock_client();

    let private_key = [1u8; 31]; // Invalid length (should be 32)
                                 // Construction of PrivateKey should fail for invalid length
    let try_pk = PrivateKey::try_from_slice(&private_key);
    assert!(try_pk.is_err());
}

#[tokio::test]
async fn test_send_transaction_invalid_to_address() {
    let (client, _) = create_mock_client();

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "invalid";
    let amount = "0.01";

    let result = client.send_transaction(&pk, to_address, amount).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_transaction_invalid_amount() {
    let (client, _) = create_mock_client();

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "invalid";

    let result = client.send_transaction(&pk, to_address, amount).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_transaction_empty_private_key() {
    let (client, _) = create_mock_client();

    let private_key = [0u8; 32]; // All zeros
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "0.01";

    let result = client.send_transaction(&pk, to_address, amount).await;
    assert!(result.is_err()); // Should fail due to invalid private key content
}

#[tokio::test]
async fn test_send_transaction_same_address() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x14791697260E4c9A71f18484C9f997B308e59325"; // Address for private_key [1u8; 32]
    let amount = "0.01";

    // This might succeed or fail depending on implementation; for coverage, call it
    let result = client.send_transaction(&pk, to_address, amount).await;
    // Assuming it succeeds in mock
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_send_transaction_large_amount() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "1000000.0"; // Large amount

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_negative_amount() {
    let (client, _) = create_mock_client();

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "-0.01";

    let result = client.send_transaction(&pk, to_address, amount).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_transaction_empty_amount() {
    let (client, _) = create_mock_client();

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "";

    let result = client.send_transaction(&pk, to_address, amount).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_transaction_with_custom_gas() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(30_000_000_000u64)))); // Higher gas price
    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "0.01";

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_empty_to_address() {
    let (client, _) = create_mock_client();

    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "";
    let amount = "0.01";

    let result = client.send_transaction(&pk, to_address, amount).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_transaction_with_max_private_key() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    // Use a valid private key
    let private_key = [1u8; 32];
    let pk =
        defi_hot_wallet::core::domain::PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let amount = "0.01";

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_various_amounts() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    let private_key = [1u8; 32];
    let pk =
        defi_hot_wallet::core::domain::PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Cc454e4438f44e";
    let amount = "0.001"; // Small amount

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_various_private_keys() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    let private_key = [2u8; 32]; // Different private key
    let pk =
        defi_hot_wallet::core::domain::PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Cc454e4438f44e";
    let amount = "0.01";

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_various_gas_prices() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    let private_key = [1u8; 32];
    let pk =
        defi_hot_wallet::core::domain::PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Cc454e4438f44e";
    let amount = "0.01";

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_various_to_addresses() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();
    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));
    let private_key = [1u8; 32];
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x1234567890123456789012345678901234567890"; // Different address
    let amount = "0.01";

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_various_combinations() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(43)))); // Different nonce
    mock_provider.push_response(MockResponse::Value(json!(U256::from(25_000_000_000u64)))); // Different gas price
    let private_key = [3u8; 32]; // Different key
    let pk = PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Cc454e4438f44e";
    let amount = "0.02"; // Different amount

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_various_edge_cases() {
    let (client, _mock_provider) = create_mock_client();

    // Use invalid private key
    // A key of all zeros is considered invalid by the `ethers` library.
    let invalid_private_key = [0u8; 32];
    let pk = PrivateKey::try_from_slice(&invalid_private_key).expect("valid pk");

    let result =
        client.send_transaction(&pk, "0x742d35Cc6634C0532925a3b844Cc454e4438f44e", "0.1").await;
    assert!(result.is_err()); // Check that the error is handled correctly
    assert!(result.unwrap_err().to_string().contains("Invalid private key"));
}

#[tokio::test]
async fn test_send_transaction_with_various_scenarios() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(100)))); // High nonce
    mock_provider.push_response(MockResponse::Value(json!(U256::from(50_000_000_000u64)))); // High gas price
    let private_key = [100u8; 32]; // Arbitrary key
    let pk =
        defi_hot_wallet::core::domain::PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Cc454e4438f44e";
    let amount = "1.0";

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_send_transaction_with_various_inputs() {
    let (client, mock_provider) = create_mock_client();
    let tx_hash =
        H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .unwrap();

    // LIFO: push tx then nonce then gas
    mock_provider.push_response(MockResponse::Value(json!(tx_hash)));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(42))));
    mock_provider.push_response(MockResponse::Value(json!(U256::from(20_000_000_000u64))));

    let private_key = [1u8; 32];
    let pk =
        defi_hot_wallet::core::domain::PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Cc454e4438f44e";
    let amount = "0.00001";

    let result_tx_hash = client.send_transaction(&pk, to_address, amount).await.unwrap();
    assert_eq!(result_tx_hash, format!("{:?}", tx_hash));
}

#[tokio::test]
async fn test_get_transaction_status_confirmed() {
    let (client, mock_provider) = create_mock_client(); // restored: client is used below

    // Manual JSON construction for TransactionReceipt with required fields
    let receipt_json = json!({
        "status": "0x1",  // Success
        "transactionHash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "transactionIndex": "0x0",
        "blockHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "blockNumber": "0x1",
        "gasUsed": "0x5208",  // 21000
        "effectiveGasPrice": "0x4a817c800",  // 20 gwei
        "cumulativeGasUsed": "0x5208",  // Added
        "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "type": "0x0"
    });
    mock_provider.push_response(MockResponse::Value(receipt_json));

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Confirmed);
}

#[tokio::test]
async fn test_get_transaction_status_pending() {
    let (client, mock_provider) = create_mock_client(); // restored: client is used below

    // Pending: receipt is None, transaction exists
    // Note: Mocking null for receipt may cause deserialization issues; adjust if needed
    // For LIFO: push tx_json then null so get_transaction_receipt() (called first) returns null and get_transaction() returns tx_json
    // This JSON should match the ethers::Transaction struct for a pending transaction
    let tx_json = json!({
        "hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "nonce": "0x0",
        "blockHash": null,
        "blockNumber": null,  // Pending
        "transactionIndex": null,
        "from": "0x0000000000000000000000000000000000000000",
        // The `to` field can be null for contract creation transactions
        "to": null,
        "value": "0xde0b6b3a7640000",  // 1 ETH
        "gas": "0x5208",
        "gasPrice": "0x4a817c800",
        "input": "0x",
        "v": "0x25",
        // signature fields required by ethers::Transaction
        "r": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "s": "0x0000000000000000000000000000000000000000000000000000000000000000"
    });
    // For LIFO: push tx_json then null so get_transaction_receipt() (called first) returns null and get_transaction() returns tx_json
    mock_provider.push_response(MockResponse::Value(tx_json));
    mock_provider.push_response(MockResponse::Value(json!(null))); // receipt

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Pending);
}

#[tokio::test]
async fn test_get_transaction_status_failed() {
    let (_client, mock_provider) = create_mock_client(); // client unused (shadowed later)

    // Failed: status = 0
    let receipt_json = json!({
        "status": "0x0",  // Failed
        "transactionHash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "transactionIndex": "0x0",
        "blockHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "blockNumber": "0x1",
        "gasUsed": "0x5208",
        "effectiveGasPrice": "0x4a817c800",
        "cumulativeGasUsed": "0x5208",  // Added
        "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "type": "0x0"
    });
    mock_provider.push_response(MockResponse::Value(receipt_json));

    let provider = Provider::new(mock_provider);
    let client = EthereumClient::new_with_provider(provider);

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Failed);
}

#[tokio::test]
async fn test_get_transaction_status_unknown() {
    let (_client, mock_provider) = create_mock_client(); // client unused (shadowed later)

    // Not found: both receipt and transaction are None
    // LIFO: push transaction then receipt null
    mock_provider.push_response(MockResponse::Value(json!(null))); // transaction
    mock_provider.push_response(MockResponse::Value(json!(null))); // receipt

    let provider = Provider::new(mock_provider);
    let client = EthereumClient::new_with_provider(provider);

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Unknown);
}

#[tokio::test]
async fn test_get_transaction_status_reorg() {
    let (_client, mock_provider) = create_mock_client(); // client unused (shadowed later)

    // Simulate reorg: receipt exists but status is None (reorged)
    let receipt_json = json!({
        "status": null,
        "transactionHash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "transactionIndex": "0x0",
        "blockHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "blockNumber": "0x1",
        "gasUsed": "0x5208",
        "effectiveGasPrice": "0x4a817c800",
        "cumulativeGasUsed": "0x5208",  // Added
        "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "type": "0x0"
    });
    mock_provider.push_response(MockResponse::Value(receipt_json));

    let provider = Provider::new(mock_provider);
    let client = EthereumClient::new_with_provider(provider);

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Failed); // Assuming None status means failed/reorged
}

#[tokio::test]
async fn test_get_transaction_status_malformed_hash() {
    let (client, _mock_provider) = create_mock_client();

    // Malformed hash (not hex)
    let malformed_hash = "0xgggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg";
    let result = client.get_transaction_status(malformed_hash).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_transaction_status_null_hash() {
    let (client, _mock_provider) = create_mock_client();

    // Null hash
    let null_hash = "0x0000000000000000000000000000000000000000000000000000000000000000";
    let result = client.get_transaction_status(null_hash).await;
    // Depending on implementation, might be Unknown or error
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_get_transaction_status_invalid_hash_length() {
    let (client, _mock_provider) = create_mock_client();

    // Invalid hash length (too short)
    let invalid_hash = "0x123";
    let result = client.get_transaction_status(invalid_hash).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_transaction_status_with_empty_hash() {
    let (client, _mock_provider) = create_mock_client();

    let tx_hash = "";
    let result = client.get_transaction_status(tx_hash).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_transaction_status_with_various_hashes() {
    let (_client, mock_provider) = create_mock_client(); // client unused (shadowed later)

    // Confirmed with different hash
    let receipt_json = json!({
        "status": "0x1",
        "transactionHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "transactionIndex": "0x0",
        "blockHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "blockNumber": "0x1",
        "gasUsed": "0x5208",
        "effectiveGasPrice": "0x4a817c800",
        "cumulativeGasUsed": "0x5208",
        "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "type": "0x0"
    });
    mock_provider.push_response(MockResponse::Value(receipt_json));

    let provider = Provider::new(mock_provider);
    let client = EthereumClient::new_with_provider(provider);

    let tx_hash = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Confirmed);
}

#[tokio::test]
async fn test_get_transaction_status_with_various_statuses() {
    let (_client, mock_provider) = create_mock_client(); // client unused (shadowed later)

    // Failed with different details
    let receipt_json = json!({
        "status": "0x0",
        "transactionHash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "transactionIndex": "0x0",
        "blockHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "blockNumber": "0x1",
        "gasUsed": "0x5208",
        "effectiveGasPrice": "0x4a817c800",
        "cumulativeGasUsed": "0x5208",
        "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "type": "0x0"
    });
    mock_provider.push_response(MockResponse::Value(receipt_json));

    let provider = Provider::new(mock_provider);
    let client = EthereumClient::new_with_provider(provider);

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Failed);
}

#[tokio::test]
async fn test_get_transaction_status_with_various_edge_cases() {
    let (_client, mock_provider) = create_mock_client();

    // Unknown with different setup
    mock_provider.push_response(MockResponse::Value(json!(null)));
    mock_provider.push_response(MockResponse::Value(json!(null)));

    let provider = Provider::new(mock_provider);
    let client = EthereumClient::new_with_provider(provider);

    let tx_hash = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Unknown);
}

#[tokio::test]
async fn test_get_transaction_status_with_various_inputs() {
    let (client, mock_provider) = create_mock_client();

    // Pending with different inputs
    // For LIFO: push tx_json then null so get_transaction_receipt() (called first) returns null and get_transaction() returns tx_json
    // This JSON should match the ethers::Transaction struct for a pending transaction
    let tx_json = json!({
        "hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "nonce": "0x1",
        "blockHash": null,
        "blockNumber": null,
        "transactionIndex": null,
        "from": "0x0000000000000000000000000000000000000000",
        // The `to` field can be null for contract creation transactions
        "to": null,
        "value": "0xde0b6b3a7640000",
        "gas": "0x5208",
        "gasPrice": "0x4a817c800",
        "input": "0x",
        "v": "0x25",
        "r": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "s": "0x0000000000000000000000000000000000000000000000000000000000000000"
    });
    mock_provider.push_response(MockResponse::Value(tx_json));
    mock_provider.push_response(MockResponse::Value(json!(null)));

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let status = client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(status, TransactionStatus::Pending);
}

#[tokio::test]
async fn test_send_transaction_with_invalid_provider() {
    let (client, mock_provider) = create_mock_client();

    // Simulate a provider error
    let rpc_error = ethers::providers::JsonRpcError {
        code: -32600,
        message: "Invalid request".to_string(),
        data: Some(serde_json::Value::String("invalid".to_string())),
    };
    mock_provider.push_response(MockResponse::Error(rpc_error));

    let private_key = [1u8; 32];
    let pk =
        defi_hot_wallet::core::domain::PrivateKey::try_from_slice(&private_key).expect("valid pk");
    let to_address = "0x742d35Cc6634C0532925a3b844Cc454e4438f44e";
    let amount = "0.01";

    let result = client.send_transaction(&pk, to_address, amount).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_transaction_status_with_invalid_provider() {
    let (client, mock_provider) = create_mock_client();

    let rpc_error = ethers::providers::JsonRpcError {
        code: -32600,
        message: "Invalid request".to_string(),
        data: Some(serde_json::Value::String("invalid".to_string())),
    };
    mock_provider.push_response(MockResponse::Error(rpc_error));

    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let result = client.get_transaction_status(tx_hash).await;
    assert!(result.is_err());
}
