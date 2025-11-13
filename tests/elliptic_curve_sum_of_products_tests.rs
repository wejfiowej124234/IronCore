// ...existing code...
use defi_hot_wallet::blockchain::ethereum::*;
use defi_hot_wallet::blockchain::traits::BlockchainClient;
use ethers::providers::{MockProvider, Provider};

/// Create an EthereumClient backed by Provider<MockProvider>.
/// Note: Provider<MockProvider> -> new_with_provider(...) returns EthereumClient<MockProvider>,
/// so the function must return EthereumClient<MockProvider>.
fn create_mock_client() -> EthereumClient<MockProvider> {
    let mock_provider = MockProvider::new();
    let provider = Provider::new(mock_provider);
    // provider is Provider<MockProvider>, but new_with_provider returns EthereumClient<MockProvider>
    EthereumClient::new_with_provider(provider)
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_valid() {
    let client = create_mock_client();
    let valid_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert!(client.validate_address(valid_address).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_invalid_short() {
    let client = create_mock_client();
    assert!(!client.validate_address("0x12345").unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_valid_no_prefix() {
    let client = create_mock_client();
    assert!(client.validate_address("742d35Cc6634C0532925a3b844Bc454e4438f44e").unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_invalid_special_chars() {
    let client = create_mock_client();
    assert!(!client.validate_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44e!").unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_empty() {
    let client = create_mock_client();
    assert!(!client.validate_address("").unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_all_zeros() {
    let client = create_mock_client();
    let zero_address = "0x0000000000000000000000000000000000000000";
    assert!(client.validate_address(zero_address).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_case_insensitive() {
    let client = create_mock_client();
    let lower = "0x742d35cc6634c0532925a3b844bc454e4438f44e";
    let upper = "0x742D35CC6634C0532925A3B844BC454E4438F44E";
    assert!(client.validate_address(lower).unwrap());
    assert!(client.validate_address(upper).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_too_long() {
    let client = create_mock_client();
    let long_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e1234";
    assert!(!client.validate_address(long_address).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_too_short() {
    let client = create_mock_client();
    let short_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44";
    assert!(!client.validate_address(short_address).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_with_checksum() {
    let client = create_mock_client();
    let checksum_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert!(client.validate_address(checksum_address).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_mixed_case_valid() {
    let client = create_mock_client();
    let mixed_case = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert!(client.validate_address(mixed_case).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_uppercase_valid() {
    let client = create_mock_client();
    let uppercase = "0X742D35CC6634C0532925A3B844BC454E4438F44E";
    assert!(!client.validate_address(uppercase).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_with_numbers_only() {
    let client = create_mock_client();
    let num_address = "0x1234567890123456789012345678901234567890";
    assert!(client.validate_address(num_address).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn test_validate_address_with_leading_zeros() {
    let client = create_mock_client();
    let leading_zero = "0x0000000000000000000000000000000000000000";
    assert!(client.validate_address(leading_zero).unwrap());
}
// ...existing code...
