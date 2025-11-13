//! 账户锁定机制
//!
//! 防止暴力破解攻击的账户锁定功能

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 账户锁定配置
#[derive(Debug, Clone)]
pub struct LockoutConfig {
    /// 最大failed尝试次数
    pub max_attempts: u32,
    /// 锁定持续时间（秒）
    pub lockout_duration_secs: u64,
    /// 尝试计数重置时间（秒）
    pub reset_duration_secs: u64,
}

impl Default for LockoutConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,           // 5次failed后锁定
            lockout_duration_secs: 900, // 锁定15 minutes
            reset_duration_secs: 300,   // 5 minutes后重置计数
        }
    }
}

/// 登录尝试记录
#[derive(Debug, Clone)]
struct LoginAttempt {
    /// failed次数
    failed_count: u32,
    /// 最后一次尝试时间
    last_attempt: Instant,
    /// 锁定到期时间
    locked_until: Option<Instant>,
}

/// 账户锁定管理器
pub struct AccountLockout {
    /// 配置
    config: LockoutConfig,
    /// 尝试记录 (email/username -> LoginAttempt)
    attempts: Arc<RwLock<HashMap<String, LoginAttempt>>>,
}

impl AccountLockout {
    /// 创建新的账户锁定管理器
    pub fn new(config: LockoutConfig) -> Self {
        Self {
            config,
            attempts: Arc::new(RwLock::new(HashMap::with_capacity(1000))),
        }
    }

    /// 使用默认配置创建
    pub fn with_default() -> Self {
        Self::new(LockoutConfig::default())
    }

    /// check账户是否被锁定
    ///
    /// # Arguments
    /// * `identifier` - user标识（email或username）
    ///
    /// # Returns
    /// * `Ok(())` - 账户未锁定
    /// * `Err(remaining_secs)` - 账户已锁定，返回剩余锁定秒数
    pub async fn check_lockout(&self, identifier: &str) -> Result<(), u64> {
        let mut attempts = self.attempts.write().await;
        let now = Instant::now();

        if let Some(attempt) = attempts.get_mut(identifier) {
            // check是否仍在锁定期
            if let Some(locked_until) = attempt.locked_until {
                if now < locked_until {
                    let remaining = locked_until.duration_since(now).as_secs();
                    return Err(remaining);
                } else {
                    // 锁定期已过，重置状态
                    attempt.locked_until = None;
                    attempt.failed_count = 0;
                    attempt.last_attempt = now;
                }
            }

            // check是否需要重置failed计数
            let elapsed = now.duration_since(attempt.last_attempt);
            if elapsed > Duration::from_secs(self.config.reset_duration_secs) {
                attempt.failed_count = 0;
                attempt.last_attempt = now;
            }
        }

        Ok(())
    }

    /// 记录登录failed
    ///
    /// # Arguments
    /// * `identifier` - user标识
    ///
    /// # Returns
    /// * `Ok(remaining_attempts)` - 剩余尝试次数
    /// * `Err(lockout_secs)` - 账户已被锁定，返回锁定秒数
    pub async fn record_failure(&self, identifier: &str) -> Result<u32, u64> {
        let mut attempts = self.attempts.write().await;
        let now = Instant::now();

        let attempt = attempts.entry(identifier.to_string()).or_insert(LoginAttempt {
            failed_count: 0,
            last_attempt: now,
            locked_until: None,
        });

        // 增加failed计数
        attempt.failed_count += 1;
        attempt.last_attempt = now;

        // check是否达到锁定阈值
        if attempt.failed_count >= self.config.max_attempts {
            let locked_until = now + Duration::from_secs(self.config.lockout_duration_secs);
            attempt.locked_until = Some(locked_until);
            
            tracing::warn!(
                "Account locked due to too many failed attempts: identifier={}, attempts={}, lockout_duration_secs={}",
                identifier,
                attempt.failed_count,
                self.config.lockout_duration_secs
            );

            return Err(self.config.lockout_duration_secs);
        }

        let remaining = self.config.max_attempts - attempt.failed_count;
        Ok(remaining)
    }

    /// 记录登录success（重置failed计数）
    ///
    /// # Arguments
    /// * `identifier` - user标识
    pub async fn record_success(&self, identifier: &str) {
        let mut attempts = self.attempts.write().await;
        
        if let Some(attempt) = attempts.get_mut(identifier) {
            attempt.failed_count = 0;
            attempt.locked_until = None;
            
            tracing::debug!("Login successful, reset failure count for: {}", identifier);
        }
    }

    /// 手动解锁账户（管理员功能）
    ///
    /// # Arguments
    /// * `identifier` - user标识
    pub async fn unlock_account(&self, identifier: &str) {
        let mut attempts = self.attempts.write().await;
        
        if let Some(attempt) = attempts.get_mut(identifier) {
            attempt.failed_count = 0;
            attempt.locked_until = None;
            
            tracing::info!("Account manually unlocked: {}", identifier);
        }
    }

    /// fetchfailed尝试信息
    ///
    /// # Arguments
    /// * `identifier` - user标识
    ///
    /// # Returns
    /// (failed_count, is_locked, remaining_lockout_secs)
    pub async fn get_attempt_info(&self, identifier: &str) -> (u32, bool, Option<u64>) {
        let attempts = self.attempts.read().await;
        let now = Instant::now();

        if let Some(attempt) = attempts.get(identifier) {
            if let Some(locked_until) = attempt.locked_until {
                if now < locked_until {
                    let remaining = locked_until.duration_since(now).as_secs();
                    return (attempt.failed_count, true, Some(remaining));
                }
            }
            (attempt.failed_count, false, None)
        } else {
            (0, false, None)
        }
    }

    /// 清理过期的记录（定期维护）
    pub async fn cleanup_expired(&self) {
        let mut attempts = self.attempts.write().await;
        let now = Instant::now();
        let cleanup_threshold = Duration::from_secs(self.config.reset_duration_secs * 2);

        attempts.retain(|_, attempt| {
            // 保留仍在锁定期的记录
            if let Some(locked_until) = attempt.locked_until {
                if now < locked_until {
                    return true;
                }
            }

            // 保留最近有活动的记录
            now.duration_since(attempt.last_attempt) < cleanup_threshold
        });

        tracing::debug!("Lockout cleanup completed, remaining entries: {}", attempts.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lockout_after_max_attempts() {
        let config = LockoutConfig {
            max_attempts: 3,
            lockout_duration_secs: 10,
            reset_duration_secs: 5,
        };
        let lockout = AccountLockout::new(config);

        // 前两次failed
        assert!(lockout.record_failure("test@example.com").await.is_ok());
        assert!(lockout.record_failure("test@example.com").await.is_ok());

        // 第三次failed应该触发锁定
        let result = lockout.record_failure("test@example.com").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 10);

        // check锁定状态
        let check_result = lockout.check_lockout("test@example.com").await;
        assert!(check_result.is_err());
    }

    #[tokio::test]
    async fn test_success_resets_counter() {
        let lockout = AccountLockout::with_default();

        // 记录failed
        lockout.record_failure("test@example.com").await.ok();
        lockout.record_failure("test@example.com").await.ok();

        // 记录success
        lockout.record_success("test@example.com").await;

        // 再次failed，应该from1start计数
        let result = lockout.record_failure("test@example.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 4); // 5 - 1 = 4 remaining
    }

    #[tokio::test]
    async fn test_manual_unlock() {
        let lockout = AccountLockout::with_default();

        // 触发锁定
        for _ in 0..5 {
            lockout.record_failure("test@example.com").await.ok();
        }

        // validate已锁定
        assert!(lockout.check_lockout("test@example.com").await.is_err());

        // 手动解锁
        lockout.unlock_account("test@example.com").await;

        // validate已解锁
        assert!(lockout.check_lockout("test@example.com").await.is_ok());
    }
}

