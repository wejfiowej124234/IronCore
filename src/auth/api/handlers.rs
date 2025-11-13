//! API请求处理器（轻量）

use axum::{
    extract::{Json, State, Query},
    http::{HeaderMap, StatusCode},
};
use std::sync::Arc;
use tracing::info;

use crate::auth::{
    types::*,
    errors::AuthError,
    AuthService,
};

/// user注册处理器
pub async fn register(
    State(service): State<Arc<AuthService>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AuthError> {
    info!("API: 收到Registration request: email={}", req.email);
    let response = service.register(req).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// user登录处理器
pub async fn login(
    State(service): State<Arc<AuthService>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    info!("API: 收到Login request: email={}", req.email);
    let response = service.login(req).await?;
    Ok(Json(response))
}

/// Google OAuth处理器
pub async fn google_auth(
    State(service): State<Arc<AuthService>>,
    Json(req): Json<GoogleAuthRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    info!("API: 收到Google OAuth请求");
    let response = service.google_login(req.id_token).await?;
    Ok(Json(response))
}

/// fetch当前User information处理器
pub async fn get_current_user(
    State(service): State<Arc<AuthService>>,
    headers: HeaderMap,
) -> Result<Json<User>, AuthError> {
    info!("API: 收到fetchUser information请求");
    
    // from请求头提取token
    let token = extract_token_from_headers(&headers)?;
    
    let user = service.get_user_by_token(&token).await?;
    Ok(Json(user))
}

/// 修改Password处理器
pub async fn change_password(
    State(service): State<Arc<AuthService>>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, AuthError> {
    info!("API: 收到修改Password请求");
    
    // from请求头提取token
    let token = extract_token_from_headers(&headers)?;
    
    service.change_password(&token, &req.old_password, &req.new_password).await?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// user登出处理器
pub async fn logout(
    headers: HeaderMap,
) -> Result<StatusCode, AuthError> {
    info!("API: 收到登出请求");
    
    // from请求头提取token（validatetoken有效性）
    let _token = extract_token_from_headers(&headers)?;
    
    // TODO: 实现token黑名单或令牌吊销机制
    // 目前简化实现：客户端Deletetoken即可
    
    Ok(StatusCode::NO_CONTENT)
}

/// check名称可用性处理器
pub async fn check_name(
    State(service): State<Arc<AuthService>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<NameCheckResponse>, AuthError> {
    let name = params.get("name").ok_or(AuthError::ValidationError("缺少name参数".to_string()))?;
    
    info!("API: check名称可用性: name={}", name);
    
    let response = service.check_name_available(name).await?;
    Ok(Json(response))
}

// ========== 辅助函数 ==========

/// from请求头提取Token
fn extract_token_from_headers(headers: &HeaderMap) -> Result<String, AuthError> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| {
            auth.strip_prefix("Bearer ").map(|s| s.to_string())
        })
        .ok_or(AuthError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str("Bearer token_123").unwrap()
        );
        
        let token = extract_token_from_headers(&headers).unwrap();
        assert_eq!(token, "token_123");
    }

    #[test]
    fn test_extract_token_missing() {
        let headers = HeaderMap::new();
        let result = extract_token_from_headers(&headers);
        assert!(result.is_err());
    }
}

