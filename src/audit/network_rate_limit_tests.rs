// filepath: c:\Users\plant\Desktop\Rust鍖哄潡閾綷Defi-Hot-wallet-Rust\tests\network_rate_limit_tests.rs

use defi_hot_wallet::network::rate_limit::*;

#[test]
fn test_rate_limit_basic() {
    let limiter = RateLimiter::new(10, std::time::Duration::from_secs(60));
    assert!(limiter.allow());
}
