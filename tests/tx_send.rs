use defi_hot_wallet::mvp::{confirm_transaction, send_transaction};

#[test]
fn tx_send_and_confirm() {
    let tx_hash = send_transaction("test_wallet", Some(100)).unwrap();
    let confirmed = confirm_transaction(tx_hash).expect("confirm ok");
    assert!(confirmed);
}
