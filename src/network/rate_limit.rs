//! src/network/rate_limit.rs
//!
//! Provides rate limiting functionality for network requests.

use governor::{Quota, RateLimiter as GovernorRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

/// A rate limiter for network requests, wrapping the `governor` crate.
#[derive(Clone)]
pub struct RateLimiter {
    // Using an Arc to allow the limiter to be shared across threads.
    limiter: Arc<
        GovernorRateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
        >,
    >,
}

impl RateLimiter {
    /// Creates a new rate limiter.
    ///
    /// # Arguments
    /// * `requests` - The number of requests allowed per time period.
    /// * `period` - The time period for the requests.
    pub fn new(requests: u32, period: Duration) -> Self {
        let quota =
            Quota::with_period(period).unwrap().allow_burst(NonZeroU32::new(requests).unwrap());
        Self { limiter: Arc::new(GovernorRateLimiter::direct(quota)) }
    }

    /// Checks if a request is allowed under the current rate limit.
    pub fn allow(&self) -> bool {
        self.limiter.check().is_ok()
    }
}
