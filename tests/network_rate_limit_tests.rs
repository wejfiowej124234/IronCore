//! tests/network_rate_limit_tests.rs
//!
//! Tests for `src/network/rate_limit.rs`
//! - ensure RateLimiter construction and basic allow/deny behavior
//! - verify cloned limiter shares state (if implementation uses Arc/Shared state)

use defi_hot_wallet::network::rate_limit::RateLimiter;
use std::time::Duration;

#[test]
fn test_rate_limiter_new_and_initial_allow() {
    // Create a limiter allowing 10 requests per 1 second window.
    let limiter = RateLimiter::new(10, Duration::from_secs(1));
    assert!(limiter.allow(), "First request should be allowed");
}

#[test]
fn test_rate_limiter_exceeds_limit() {
    // Create a limiter that only allows 1 request per 200ms window.
    let limiter = RateLimiter::new(1, Duration::from_millis(200));

    // First request must be allowed.
    assert!(limiter.allow(), "The first request should be allowed");

    // Immediate second request should be denied because quota is exhausted.
    assert!(!limiter.allow(), "The second request should be denied as it exceeds the rate limit");
}

#[test]
fn test_rate_limiter_clone_shares_state() {
    // If RateLimiter::clone shares internal state (Arc-like), consuming on one clone
    // should affect the other. This test documents that expected behavior.
    let limiter1 = RateLimiter::new(1, Duration::from_millis(200));
    let limiter2 = limiter1.clone();

    // Use limiter1 first - allowed.
    assert!(limiter1.allow(), "First request on limiter1 should be allowed");

    // Now limiter2 should see the quota consumed and deny.
    assert!(!limiter2.allow(), "Request on cloned limiter2 should be denied as the quota is used");
}
