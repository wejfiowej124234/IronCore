//! Result扩展工具
//! 
//! 提供安全的Result处理方法，替代unwrap/expect

use crate::core::errors::WalletError;

/// Result扩展trait
pub trait ResultExt<T> {
    /// 安全地fetch值，failed时记录日志并返回默认值
    fn unwrap_or_log(self, default: T, context: &str) -> T;
    
    /// 安全地fetch值，failed时记录日志并返回error
    fn expect_or_log(self, context: &str) -> Result<T, WalletError>;
}

impl<T> ResultExt<T> for Option<T> {
    fn unwrap_or_log(self, default: T, context: &str) -> T {
        match self {
            Some(v) => v,
            None => {
                tracing::warn!("unwrap failed: {}", context);
                default
            }
        }
    }
    
    fn expect_or_log(self, context: &str) -> Result<T, WalletError> {
        self.ok_or_else(|| {
            tracing::error!("expect failed: {}", context);
            WalletError::InternalError(format!("Missing value: {}", context))
        })
    }
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn unwrap_or_log(self, default: T, context: &str) -> T {
        match self {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("unwrap failed in {}: {}", context, e);
                default
            }
        }
    }
    
    fn expect_or_log(self, context: &str) -> Result<T, WalletError> {
        self.map_err(|e| {
            tracing::error!("expect failed in {}: {}", context, e);
            WalletError::InternalError(format!("{}: {}", context, e))
        })
    }
}

/// 安全的unwrap宏
#[macro_export]
macro_rules! safe_unwrap {
    ($expr:expr, $default:expr) => {
        $crate::core::result_ext::ResultExt::unwrap_or_log($expr, $default, stringify!($expr))
    };
}

/// 安全的expect宏
#[macro_export]
macro_rules! safe_expect {
    ($expr:expr) => {
        $crate::core::result_ext::ResultExt::expect_or_log($expr, stringify!($expr))
    };
}

