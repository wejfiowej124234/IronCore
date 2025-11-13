//! 认证error类型定义

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// 认证error
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Weak password: {0}")]
    WeakPassword(String),

    #[error("Password mismatch")]
    PasswordMismatch,

    #[error("Email already exists")]
    EmailExists,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl AuthError {
    /// fetchHTTP状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidEmail 
            | Self::WeakPassword(_) 
            | Self::PasswordMismatch 
            | Self::ValidationError(_)
            | Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            
            Self::EmailExists => StatusCode::CONFLICT,
            
            Self::InvalidCredentials 
            | Self::InvalidToken 
            | Self::TokenExpired 
            | Self::Unauthorized => StatusCode::UNAUTHORIZED,
            
            Self::UserNotFound => StatusCode::NOT_FOUND,
            
            Self::OAuthError(_) 
            | Self::StorageError(_) 
            | Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// fetcherror代码（用于前端判断）
    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidEmail => "invalid_email",
            Self::WeakPassword(_) => "weak_password",
            Self::PasswordMismatch => "password_mismatch",
            Self::EmailExists => "email_exists",
            Self::InvalidCredentials => "invalid_credentials",
            Self::UserNotFound => "not_found",
            Self::InvalidToken => "invalid_token",
            Self::TokenExpired => "token_expired",
            Self::Unauthorized => "unauthorized",
            Self::OAuthError(_) => "oauth_error",
            Self::StorageError(_) => "storage_error",
            Self::InternalError(_) => "server_error",
            Self::ValidationError(_) => "validation_error",
            Self::InvalidInput(_) => "invalid_input",
        }
    }

    /// fetchuser友好的error消息
    pub fn user_message(&self) -> &str {
        match self {
            Self::InvalidEmail => "Email格式不正确",
            Self::WeakPassword(_) => "Password强度不足",
            Self::PasswordMismatch => "两次输入的Password不一致",
            Self::EmailExists => "该Email已被注册",
            Self::InvalidCredentials => "Email或Passworderror",
            Self::UserNotFound => "user不存在",
            Self::InvalidToken => "无效的Access token",
            Self::TokenExpired => "Access token已过期，请重新登录",
            Self::Unauthorized => "未授权访问",
            Self::OAuthError(_) => "第三方登录failed，请稍后重试",
            Self::StorageError(_) => "数据存储error",
            Self::InternalError(_) => "系统error，请稍后重试",
            Self::ValidationError(_) => "输入validatefailed",
            Self::InvalidInput(_) => "输入参数无效",
        }
    }
}

/// 实现 IntoResponse，使 AuthError 可以直接作为 Axum 响应
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code();
        let message = self.user_message();
        
        let body = Json(json!({
            "code": code,
            "message": message,
            "details": self.to_string(),
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(AuthError::InvalidEmail.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AuthError::EmailExists.status_code(), StatusCode::CONFLICT);
        assert_eq!(AuthError::InvalidCredentials.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::UserNotFound.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(AuthError::InvalidEmail.user_message(), "Email格式不正确");
        assert_eq!(AuthError::EmailExists.user_message(), "该Email已被注册");
    }
}

