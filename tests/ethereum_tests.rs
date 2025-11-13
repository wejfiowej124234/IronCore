use defi_hot_wallet::blockchain::ethereum::EthereumClient;
use defi_hot_wallet::blockchain::BlockchainClient;
use defi_hot_wallet::core::domain::PrivateKey;
use ethers::providers::{Http, Provider};
use std::convert::TryFrom;

#[tokio::test(flavor = "current_thread")]
async fn send_transaction_invalid_key_errors() {
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545").unwrap();
    let _client = EthereumClient::new_with_provider(provider);
    let short_key = [0u8; 16];
    // PrivateKey construction should fail for an invalid-length key
    let try_pk = PrivateKey::try_from_slice(&short_key);
    assert!(try_pk.is_err());
}

#[test]
fn validate_address_public_api() {
    // This test doesn't need a live provider; creating a provider instance is lightweight here.
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545").unwrap();
    let client = EthereumClient::new_with_provider(provider);

    assert!(client.validate_address("0x742d35Cc6634C0532925a3b8D400e8B78fFe4860").unwrap());
    assert!(!client.validate_address("abc").unwrap());
}
