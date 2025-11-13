//! 服务器配置常量

use std::time::Duration;

/// 并发连接限制
pub const MAX_CONCURRENCY: usize = 256;

/// 请求体大小限制
pub const MAX_BODY_SIZE: usize = 1024 * 1024; // 1MB

/// 敏感端点请求体限制
pub const MAX_SENSITIVE_BODY_SIZE: usize = 256 * 1024; // 256KB

/// 请求超时时间
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// 敏感端点超时时间
pub const SENSITIVE_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// CORS最大缓存时间
pub const CORS_MAX_AGE: Duration = Duration::from_secs(3600);

/// 速率限制配置
pub const RATE_LIMIT_REQUESTS: u32 = 10000;
pub const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(1);

