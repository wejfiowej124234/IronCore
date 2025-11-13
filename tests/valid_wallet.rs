use defi_hot_wallet::mvp::{
    bridge_assets_amount, calculate_bridge_fee, create_wallet, send_transaction,
};

#[test]
fn test_send_transaction() {
    let result = send_transaction("valid_wallet", Some(100));
    assert!(result.is_ok());
}

#[test]
fn test_create_wallet() {
    let result = create_wallet("newwallet");
    assert!(result.is_ok());
}

#[test]
fn test_bridge_assets_amount() {
    let result = bridge_assets_amount(Some("100.0"));
    assert!(result.is_ok());
}

#[test]
fn test_calculate_bridge_fee() {
    let result = calculate_bridge_fee(Some("100.0"));
    assert!(result.is_ok());
}
