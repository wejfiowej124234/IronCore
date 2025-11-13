//! tests/audit_rollback_tests.rs
//!
//! 閽堝 `src/audit/rollback.rs` 鐨勫崟鍏冩祴璇曘€?

use defi_hot_wallet::audit::rollback::*;

#[test]
fn test_rollback_new() {
    // 姝ｅ父璺緞锛氭祴璇曞垱寤烘柊鐨勫洖婊氳姹?
    let rollback = Rollback::new("tx_id_to_revert");
    assert_eq!(rollback.tx_id, "tx_id_to_revert");
}

/// 娴嬭瘯 `rollback_tx` 鍗犱綅鍑芥暟銆?
/// 杩欎釜娴嬭瘯楠岃瘉浜嗗崰浣嶅嚱鏁板綋鍓嶆€绘槸杩斿洖鎴愬姛 (`Ok(())`)锛?
/// 纭繚浜嗗嵆浣垮湪妯℃嫙瀹炵幇涓嬶紝鍏惰涓轰篃鏄彲棰勬祴鐨勩€?
#[test]
fn test_rollback_tx_function() {
    assert_eq!(rollback_tx("any_tx_id"), Ok(()));
}
