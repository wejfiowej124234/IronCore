//! tests/ops_backup_tests.rs
//!
//! 閽堝 `src/ops/backup.rs` 鐨勫崟鍏冩祴璇曘€?

use defi_hot_wallet::ops::backup::*;

#[test]
fn test_backup_create() {
    // 姝ｅ父璺緞锛氭祴璇曞垱寤烘柊鐨勫浠戒换鍔?
    let backup = Backup::new("my_precious_wallet");
    assert_eq!(backup.wallet_name, "my_precious_wallet");
}

#[test]
fn test_perform_backup_function() {
    // 姝ｅ父璺緞锛氭祴璇曞崰浣嶅嚱鏁版€绘槸鎴愬姛
    let backup = Backup::new("any_wallet");
    assert_eq!(perform_backup(&backup), Ok(()));
}
