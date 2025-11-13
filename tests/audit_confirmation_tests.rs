use defi_hot_wallet::audit::confirmation::*;

#[test]
fn test_confirmation_new() {
    let confirmation = Confirmation::new("tx_id");
    assert_eq!(confirmation.tx_id, "tx_id");
    assert!(!confirmation.is_confirmed()); // 瑕嗙洊鍒濆 confirmed = false
}

#[test]
fn test_confirmation_confirm() {
    let mut confirmation = Confirmation::new("tx_id");
    confirmation.confirm(); // 瑕嗙洊 confirm 鏂规硶
    assert!(confirmation.is_confirmed()); // 瑕嗙洊 is_confirmed 杩斿洖 true
}

#[test]
fn test_require_confirmation() {
    assert!(require_confirmation("some_op")); // 瑕嗙洊 require_confirmation 鍑芥暟
}
