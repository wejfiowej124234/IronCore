//! API Rate Limiting
//!
//! Provides IP-based rate limiting for API endpoints to prevent:
//! - Brute force attacks
//! - DDoS attacks
//! - API abuse

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::{QuantaClock, QuantaInstant},
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use std::collections::HashMap;
use serde_json::json;
use tracing::{debug, warn, error};

/// Per-IP rate limiter with TTL and LRU eviction
#[derive(Clone)]
pub struct IpRateLimiter {
    // Store a rate limiter for each IP address with last access time
    limiters: Arc<RwLock<HashMap<IpAddr, (Arc<RateLimiter<NotKeyed, InMemoryState, QuantaClock>>, Instant)>>>,
    quota: Quota,
    max_entries: usize,
    entry_ttl: Duration,
}

impl IpRateLimiter {
    /// Create a new IP-based rate limiter
    ///
    /// # Arguments
    /// * `requests_per_second` - Maximum requests per second per IP
    /// * `burst` - Maximum burst size
    ///
    /// # Panics
    /// Panics if requests_per_second or burst is 0 (should be validated at config level)
    pub fn new(requests_per_second: u32, burst: u32) -> Self {
        // ✅ 使用合理默认值代替expect
        let rps = NonZeroU32::new(requests_per_second)
            .unwrap_or_else(|| unsafe { NonZeroU32::new_unchecked(10) });
        let burst_nz = NonZeroU32::new(burst)
            .unwrap_or_else(|| unsafe { NonZeroU32::new_unchecked(20) });
        
        let quota = Quota::per_second(rps).allow_burst(burst_nz);
        
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            quota,
            max_entries: 5000,  // 降低限制，防止内存耗尽
            entry_ttl: Duration::from_secs(3600),  // 1小时TTL
        }
    }
    
    /// Create with custom limits (for testing)
    pub fn with_limits(requests_per_second: u32, burst: u32, max_entries: usize, ttl_secs: u64) -> Self {
        // ✅ 使用合理默认值代替expect
        let rps = NonZeroU32::new(requests_per_second)
            .unwrap_or_else(|| unsafe { NonZeroU32::new_unchecked(10) });
        let burst_nz = NonZeroU32::new(burst)
            .unwrap_or_else(|| unsafe { NonZeroU32::new_unchecked(20) });
        
        let quota = Quota::per_second(rps).allow_burst(burst_nz);
        
        Self {
            // ✅ 预分配容量
            limiters: Arc::new(RwLock::new(HashMap::with_capacity(max_entries.min(10000)))),
            quota,
            max_entries,
            entry_ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Check if a request from an IP is allowed
    pub fn check(&self, ip: IpAddr) -> Result<(), QuantaInstant> {
        // Get or create limiter for this IP, update last access time
        let limiter = {
            let mut write_lock = self.limiters.write();
            
            // Check if we need to evict old entries before adding new one
            if write_lock.len() >= self.max_entries && !write_lock.contains_key(&ip) {
                self.evict_oldest(&mut write_lock);
            }
            
            let entry = write_lock.entry(ip)
                .or_insert_with(|| {
                    (Arc::new(RateLimiter::direct(self.quota)), Instant::now())
                });
            
            // Update last access time
            entry.1 = Instant::now();
            entry.0.clone()
        };

        limiter.check().map_err(|not_until| not_until.earliest_possible())
    }
    
    /// Evict the oldest entry from the map
    fn evict_oldest(&self, write_lock: &mut HashMap<IpAddr, (Arc<RateLimiter<NotKeyed, InMemoryState, QuantaClock>>, Instant)>) {
        if let Some(oldest_ip) = write_lock.iter()
            .min_by_key(|(_, (_, time))| time)
            .map(|(ip, _)| *ip)
        {
            write_lock.remove(&oldest_ip);
            debug!("Evicted oldest IP from rate limiter: {}", oldest_ip);
        }
    }

    /// Clean up old limiters based on TTL (call periodically to prevent memory leak)
    ///
    /// # Security
    /// - Uses LRU eviction instead of clearing all entries
    /// - Respects TTL to remove only expired entries
    /// - Never clears all entries at once
    pub fn cleanup(&self) {
        let mut write_lock = self.limiters.write();
        let now = Instant::now();
        let initial_count = write_lock.len();
        
        // Remove expired entries (not accessed within TTL)
        write_lock.retain(|_, (_, last_access)| {
            now.duration_since(*last_access) < self.entry_ttl
        });
        
        let after_ttl_count = write_lock.len();
        
        // If still too many, remove oldest entries
        if write_lock.len() > self.max_entries {
            let mut entries: Vec<_> = write_lock.iter()
                .map(|(ip, (_, time))| (*ip, *time))
                .collect();
            entries.sort_by_key(|(_, time)| *time);
            
            let to_remove = write_lock.len() - self.max_entries;
            for (ip, _) in entries.iter().take(to_remove) {
                write_lock.remove(ip);
            }
            
            warn!(
                "Rate limiter cleanup: {} -> {} (TTL) -> {} (LRU), removed {} entries",
                initial_count, after_ttl_count, write_lock.len(), to_remove
            );
        } else if initial_count != after_ttl_count {
            debug!(
                "Rate limiter cleanup: {} -> {}, removed {} expired entries",
                initial_count, after_ttl_count, initial_count - after_ttl_count
            );
        }
    }
    
    /// Get current number of tracked IPs
    pub fn entry_count(&self) -> usize {
        self.limiters.read().len()
    }
}

/// Extract client IP from request
///
/// # Security
/// - Does NOT trust X-Forwarded-For by default (防止IP伪造)
/// - Only uses proxy headers if TRUST_PROXY_HEADERS=1 is set
/// - Returns None if IP cannot be reliably determined
/// - Logs suspicious header manipulation attempts
///
/// # Production Deployment
/// Set TRUST_PROXY_HEADERS=1 ONLY if:
/// 1. Your app is behind a trusted reverse proxy (nginx, AWS ALB, etc.)
/// 2. The proxy is configured to override X-Forwarded-For
/// 3. Direct access to your app is blocked by firewall
fn extract_client_ip(req: &Request) -> Option<IpAddr> {
    // 🔐 Security: Only trust proxy headers if explicitly enabled
    let trust_proxy = std::env::var("TRUST_PROXY_HEADERS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    
    if trust_proxy {
        // Try X-Forwarded-For (取最右侧IP，最后一个代理添加的)
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                // Get the rightmost IP (last proxy in chain)
                if let Some(last_ip) = forwarded_str.split(',').last() {
                    if let Ok(ip) = last_ip.trim().parse::<IpAddr>() {
                        // Validate it's not a private/loopback IP (防止伪造内网IP)
                        if !is_private_ip(&ip) {
                            return Some(ip);
                        } else {
                            warn!("Suspicious X-Forwarded-For with private IP: {}", forwarded_str);
                        }
                    }
                }
            }
        }

        // Try X-Real-IP (for nginx)
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    if !is_private_ip(&ip) {
                        return Some(ip);
                    } else {
                        warn!("Suspicious X-Real-IP with private IP: {}", ip_str);
                    }
                }
            }
        }
    } else {
        // Log attempts to use proxy headers when not trusted
        if req.headers().contains_key("x-forwarded-for") {
            debug!("Ignoring X-Forwarded-For (TRUST_PROXY_HEADERS not set)");
        }
    }

    // Note:需要在router中启用ConnectInfo<SocketAddr>来提取真实IP
    // 当前策略：无可靠IP时拒绝请求（更安全）
    // This is safer than falling back to 127.0.0.1
    None
}

/// Check if an IP is private/loopback (防止伪造内网IP)
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback() 
            || ipv4.is_private()
            || ipv4.is_link_local()
            || ipv4.is_broadcast()
            || ipv4.is_documentation()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
            || ipv6.is_unspecified()
            || ((ipv6.segments()[0] & 0xfe00) == 0xfc00) // ULA
        }
    }
}

/// Axum middleware for rate limiting
pub async fn rate_limit_middleware(
    limiter: Arc<IpRateLimiter>,
    req: Request,
    next: Next,
) -> Response {
    // Extract client IP
    let client_ip = match extract_client_ip(&req) {
        Some(ip) => ip,
        None => {
            // 🔐 Security: Reject requests without reliable IP instead of using fallback
            // This prevents IP spoofing and forces proper proxy configuration
            error!("Cannot determine client IP, rejecting request. Set TRUST_PROXY_HEADERS=1 if behind proxy.");
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "Cannot determine client IP",
                    "code": "IP_REQUIRED",
                    "message": "This API requires a valid client IP. If behind a proxy, configure TRUST_PROXY_HEADERS."
                }))
            ).into_response();
        }
    };

    debug!("Rate limit check for IP: {}", client_ip);

    // Check rate limit
    match limiter.check(client_ip) {
        Ok(_) => {
            // Request allowed
            next.run(req).await
        }
        Err(_not_until) => {
            // Rate limit exceeded
            let retry_after_secs = 60; // 默认60秒后重试

            warn!("Rate limit exceeded for IP: {}", client_ip);

            // Return 429 Too Many Requests
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", retry_after_secs.to_string())],
                Json(json!({
                    "error": "Too many requests",
                    "code": "RATE_LIMIT_EXCEEDED",
                    "retry_after": retry_after_secs,
                    "message": format!("Rate limit exceeded. Please retry after {} seconds.", retry_after_secs)
                }))
            ).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_quota() {
        let limiter = IpRateLimiter::new(10, 20);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // First 20 requests should be allowed (burst)
        for i in 0..20 {
            assert!(limiter.check(ip).is_ok(), "Request {} should be allowed", i);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_quota() {
        let limiter = IpRateLimiter::new(1, 5);
        let ip: IpAddr = "192.168.1.2".parse().unwrap();

        // First 5 requests should be allowed
        for i in 0..5 {
            assert!(limiter.check(ip).is_ok(), "Request {} should be allowed", i);
        }

        // 6th request should be blocked
        assert!(limiter.check(ip).is_err(), "Request should be rate limited");
    }

    #[test]
    fn test_different_ips_have_separate_limits() {
        let limiter = IpRateLimiter::new(1, 2);
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        // Exhaust quota for IP1
        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip1).is_err());

        // IP2 should still have quota
        assert!(limiter.check(ip2).is_ok());
        assert!(limiter.check(ip2).is_ok());
    }

    #[test]
    fn test_cleanup_doesnt_crash() {
        let limiter = IpRateLimiter::new(10, 20);
        
        // Add some IPs
        for i in 0..100 {
            let ip: IpAddr = format!("192.168.1.{}", i % 256).parse().unwrap();
            let _ = limiter.check(ip);
        }

        // Cleanup should not crash
        limiter.cleanup();
    }
}

