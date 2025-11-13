// filepath: c:\Users\plant\Desktop\Rust鍖哄潡閾綷Defi-Hot-wallet-Rust\tests\ops_health_tests.rs

use defi_hot_wallet::ops::health::{health_check, HealthCheck};

#[test]
fn test_health_check_struct_new_and_is_healthy() {
    // 姝ｅ父璺緞锛氭祴璇?HealthCheck::new() 鍜?is_healthy() 鏂规硶
    let health = HealthCheck::new();
    assert!(health.is_healthy(), "HealthCheck::is_healthy should return true");
}

#[test]
fn test_health_check_struct_default() {
    // 姝ｅ父璺緞锛氭祴璇?HealthCheck 鐨?Default trait 瀹炵幇
    let health = HealthCheck;
    assert!(health.is_healthy(), "Default HealthCheck instance should be healthy");
}

#[test]
fn test_standalone_health_check_function() {
    // 姝ｅ父璺緞锛氭祴璇曠嫭绔嬬殑 health_check() 鍑芥暟
    // 杩欎釜娴嬭瘯瑕嗙洊浜?`health_check` 鍑芥暟鏈韩
    assert!(health_check(), "The standalone health_check function should return true");
}
