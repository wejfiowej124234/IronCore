//! tests/application_tests.rs
//!
//! Tests for the application service layer in `src/application/application.rs`.

use defi_hot_wallet::mvp::Wallet;
use defi_hot_wallet::service::WalletService;

#[test]
fn test_service_initialization() {
    // Test that the service can be created via new() and default()
    let _service1 = WalletService::new();
    let _service2 = WalletService; // unit struct instantiation (remove .default())
                                   // The test passes if it doesn't panic.
}

#[tokio::test]
async fn test_create_wallet_service() {
    let service = WalletService::new();
    let mnemonic = "test mnemonic for wallet creation";
    let result = service.create_wallet(mnemonic).await;

    assert!(result.is_ok());
    let wallet = result.unwrap();
    let mn = wallet.mnemonic_secret();
    assert_eq!(mn.as_str(), mnemonic);
}

#[tokio::test]
async fn test_send_tx_service() {
    let service = WalletService::new();
    let wallet = Wallet::from_mnemonic("test mnemonic").expect("create wallet");
    let to_address = "0x1234567890abcdef";
    let amount = 100;

    let result = service.send_tx(&wallet, to_address, amount).await;

    assert!(result.is_ok());
    let tx = result.unwrap();
    assert_eq!(tx.to, to_address);
    assert_eq!(tx.amount, amount.to_string());
}
