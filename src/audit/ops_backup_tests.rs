// filepath: c:\Users\plant\Desktop\Rust鍖哄潡閾綷Defi-Hot-wallet-Rust\tests\ops_backup_tests.rs

use defi_hot_wallet::ops::backup::*;

#[test]
fn test_backup_basic() {
    let backup = Backup::new("wallet_name");
    assert_eq!(backup.wallet_name, "wallet_name");
}
