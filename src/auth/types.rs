//! 认证模块类型定义

use serde::{Deserialize, Serialize};

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// Email
    pub email: String,
    /// Username (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Creation time
    pub created_at: String,
    /// Last login time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<String>,
    /// Associated wallet list
    pub wallets: Vec<String>,
    /// user角色
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    /// user状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<UserStatus>,
}

/// user状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// 激活
    Active,
    /// 未激活
    Inactive,
    /// 暂停
    Suspended,
    /// Email未validate
    EmailUnverified,
}

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Email
    pub email: String,
    /// Password
    pub password: String,
    /// Confirm password（可选，兼容前端）
    #[serde(default)]
    pub confirm_password: Option<String>,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Email（主字段）
    #[serde(alias = "email_or_wallet_id")]
    pub email: String,
    /// Password
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    /// User information（放在最前面）
    pub user: User,
    /// Access token
    pub access_token: String,
    /// 兼容前端的token字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Refresh token（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Token type（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    /// 过期时间（秒，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
}

/// OAuth请求
#[derive(Debug, Deserialize)]
pub struct OAuthRequest {
    /// OAuth提供商
    pub provider: String,
    /// ID Token
    pub id_token: String,
}

/// Google OAuth请求
#[derive(Debug, Deserialize)]
pub struct GoogleAuthRequest {
    /// Google ID Token
    pub id_token: String,
}

/// 修改Password请求
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    /// 旧Password
    pub old_password: String,
    /// 新Password
    pub new_password: String,
}

/// 名称check响应
#[derive(Debug, Serialize)]
pub struct NameCheckResponse {
    /// 名称
    pub name: String,
    /// 是否可用
    pub available: bool,
    /// 提示消息
    pub message: String,
}

/// OAuthUser information
#[derive(Debug, Clone)]
pub struct OAuthUserInfo {
    /// Email
    pub email: String,
    /// user名
    pub name: Option<String>,
    /// 头像URL
    pub avatar: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "user-123".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            created_at: "2025-10-29T12:00:00Z".to_string(),
            last_login: None,
            wallets: vec!["wallet-1".to_string()],
            roles: Some(vec!["user".to_string()]),
            status: Some(UserStatus::Active),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("active"));
    }

    #[test]
    fn test_user_status() {
        assert_eq!(UserStatus::Active, UserStatus::Active);
        assert_ne!(UserStatus::Active, UserStatus::Suspended);
    }
}

