//! 认证配置管理

use serde::{Deserialize, Serialize};

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT密钥
    pub jwt_secret: String,
    
    /// Access TokenExpiration time (seconds)
    pub token_expiry: u64,
    
    /// Refresh TokenExpiration time (seconds)
    pub refresh_token_expiry: u64,
    
    /// Password配置
    pub password: PasswordConfig,
    
    /// OAuth配置
    pub oauth: OAuthConfig,
    
    /// 是否需要Emailvalidate
    pub require_email_verification: bool,
}

/// Password配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordConfig {
    /// 最小长度
    pub min_length: usize,
    
    /// 是否要求数字
    pub require_digit: bool,
    
    /// 是否要求字母
    pub require_letter: bool,
    
    /// 是否要求特殊字符
    pub require_special: bool,
    
    /// bcrypt cost
    pub bcrypt_cost: u32,
}

/// OAuth配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Google Client ID
    pub google_client_id: Option<String>,
    
    /// GitHub Client ID
    pub github_client_id: Option<String>,
    
    /// 是否启用OAuth
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev_secret_key_change_in_production".to_string()),
            token_expiry: 3600, // 1小时
            refresh_token_expiry: 86400 * 7, // 7天
            password: PasswordConfig::default(),
            oauth: OAuthConfig::default(),
            require_email_verification: false,
        }
    }
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_digit: true,
            require_letter: true,
            require_special: false, // 可选特殊字符
            bcrypt_cost: 12, // 推荐值
        }
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            google_client_id: std::env::var("GOOGLE_CLIENT_ID").ok(),
            github_client_id: std::env::var("GITHUB_CLIENT_ID").ok(),
            enabled: true,
        }
    }
}

impl AuthConfig {
    /// from环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev_secret_key".to_string()),
            token_expiry: std::env::var("TOKEN_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            refresh_token_expiry: std::env::var("REFRESH_TOKEN_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86400 * 7),
            password: PasswordConfig::from_env(),
            oauth: OAuthConfig::from_env(),
            require_email_verification: std::env::var("REQUIRE_EMAIL_VERIFICATION")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
        }
    }
}

impl PasswordConfig {
    fn from_env() -> Self {
        Self {
            min_length: std::env::var("PASSWORD_MIN_LENGTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8),
            require_digit: true,
            require_letter: true,
            require_special: false,
            bcrypt_cost: std::env::var("BCRYPT_COST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(12),
        }
    }
}

impl OAuthConfig {
    fn from_env() -> Self {
        Self {
            google_client_id: std::env::var("GOOGLE_CLIENT_ID").ok(),
            github_client_id: std::env::var("GITHUB_CLIENT_ID").ok(),
            enabled: std::env::var("OAUTH_ENABLED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuthConfig::default();
        assert_eq!(config.token_expiry, 3600);
        assert_eq!(config.password.min_length, 8);
        assert!(config.password.require_digit);
    }
}

