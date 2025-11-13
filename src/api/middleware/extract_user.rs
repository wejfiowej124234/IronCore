//! user提取中间件 - from请求头中提取当前User ID

use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use std::sync::Arc;

use crate::api::server::WalletServer;
use crate::api::types::ErrorResponse;

/// from Authorization header 提取 token
pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            if v.starts_with("Bearer ") {
                v[7..].to_string()
            } else {
                v.to_string()
            }
        })
}

/// from请求头提取当前User ID（通过validatetoken）
/// 
/// ✅ 使用SessionStorevalidatetoken并fetchuser_id
pub async fn extract_user_id_from_token(
    headers: &HeaderMap,
    state: &Arc<WalletServer>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    // 1. 提取token
    let token = extract_token(headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized: Authentication token is required".to_string(), // ✅ 英文
                code: "AUTH_REQUIRED".to_string(),
            }),
        )
    })?;
    
    // 2. ✅ fromSessionStorevalidatetoken并fetchuser_id
    match state.session_store.validate_token(&token).await {
        Ok(user_id) => {
            tracing::debug!("✅ Tokenvalidatesuccess: user_id={}", user_id);
            Ok(user_id)
        }
        Err(e) => {
            tracing::warn!("❌ Tokenvalidatefailed: {}", e);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: format!("Unauthorized: Token validation failed - {}", e), // ✅ 英文
                    code: "INVALID_TOKEN".to_string(),
                }),
            ))
        }
    }
}

/// validateuser是否有权限访问指定wallet
pub async fn verify_wallet_ownership(
    user_id: &str,
    wallet_name: &str,
    state: &Arc<WalletServer>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    // queryuser的wallet列表
    let user_wallets = state.user_db.get_user_wallets(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query user wallets: {}", e), // ✅ 英文
                    code: "DB_ERROR".to_string(),
                }),
            )
        })?;
    
    // checkwallet是否属于user
    if !user_wallets.contains(&wallet_name.to_string()) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Forbidden: You don't have permission to access this wallet".to_string(), // ✅ 英文
                code: "WALLET_ACCESS_DENIED".to_string(),
            }),
        ));
    }
    
    Ok(())
}

