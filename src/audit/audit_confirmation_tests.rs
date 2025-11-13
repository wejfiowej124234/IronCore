//! tests/audit_confirmation_tests.rs
//!
//! 閽堝 `src/audit/confirmation.rs` 鐨勫崟鍏冩祴璇曘€?

use defi_hot_wallet::audit::confirmation::*;

#[test]
fn test_confirmation_new() {
    // 姝ｅ父璺緞锛氭祴璇曟柊鍒涘缓鐨勭‘璁よ姹?
    let confirmation = Confirmation::new("tx_id_123");
    assert_eq!(confirmation.tx_id, "tx_id_123");
    // 楠岃瘉鍒濆鐘舵€佷负鏈‘璁?
    assert!(!confirmation.is_confirmed());
}

#[test]
fn test_confirmation_confirm_and_check() {
    // 姝ｅ父璺緞锛氭祴璇曠‘璁ゆ祦绋?
    let mut confirmation = Confirmation::new("tx_id_456");

    // 鍒濆鐘舵€?
    assert!(!confirmation.is_confirmed(), "Should not be confirmed initially");

    // 纭鎿嶄綔
    confirmation.confirm();

    // 楠岃瘉鏈€缁堢姸鎬?
    assert!(confirmation.is_confirmed(), "Should be confirmed after calling confirm()");
}

#[test]
fn test_require_confirmation_placeholder() {
    // 姝ｅ父璺緞锛氭祴璇曞崰浣嶅嚱鏁版€绘槸杩斿洖 true
    assert!(require_confirmation("any_operation"));
    assert!(require_confirmation(""));
}
