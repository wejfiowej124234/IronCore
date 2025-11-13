use defi_hot_wallet::mvp::{confirm_transaction, create_transaction, get_transaction_status};

#[test]
fn tx_confirm_status_changes() {
    let tx = create_transaction();
    let initial_status = get_transaction_status(tx.id.clone());
    confirm_transaction(tx.id.clone()).unwrap();
    let updated_status = get_transaction_status(tx.id);
    assert_ne!(initial_status, updated_status);
}
