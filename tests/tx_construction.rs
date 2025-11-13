use defi_hot_wallet::mvp::{construct_transaction, TransactionParams};

#[test]
fn tx_construction_builds_fields() {
    let params = TransactionParams::new("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef", 42);
    let transaction = construct_transaction(params);
    assert_eq!(transaction.amount, 42);
    assert_eq!(transaction.to, "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
}
