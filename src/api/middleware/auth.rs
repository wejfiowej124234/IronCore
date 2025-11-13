//! 认证中间件
//! 
//! 提供API请求认证功能

use axum::http::{HeaderMap, StatusCode};
use subtle::ConstantTimeEq;
use sha2::{Digest, Sha256};

/// 常量时间比较（防止时序攻击）
fn constant_time_eq_hash(a: &[u8], b: &[u8]) -> bool {
    let ha = Sha256::digest(a);
    let hb = Sha256::digest(b);
    // 直接比较digest，避免使用deprecated的as_slice
    ha.ct_eq(&hb).into()
}

/// 开发模式下的宽松认证
/// 
/// 在开发环境(DEV_MODE=1)下，如果没有提供认证头，则允许访问
pub async fn authenticate_with_dev_mode(
    headers: &HeaderMap,
    api_key: &Option<crate::security::SecretVec>,
) -> Result<(), StatusCode> {
    let is_dev = std::env::var("DEV_MODE").ok().as_deref() == Some("1");
    
    if let Some(expected) = api_key {
        // 优先checkX-API-KEY头（标准API密钥方式）
        if let Some(provided) = headers.get("X-API-KEY").or_else(|| headers.get("x-api-key")) {
            let provided = provided.to_str()
                .map_err(|_| {
                    tracing::warn!("Invalid UTF-8 in X-API-KEY header");
                    StatusCode::BAD_REQUEST
                })?;
            
            let pbytes = provided.trim().as_bytes();
            let ebytes = &**expected;
            
            if constant_time_eq_hash(pbytes, ebytes) {
                return Ok(());
            }
        }
        
        // 回退checkAuthorization头（兼容JWT或旧实现）
        if let Some(provided) = headers.get("Authorization") {
            let provided = provided.to_str()
                .map_err(|_| {
                    tracing::warn!("Invalid UTF-8 in Authorization header");
                    StatusCode::BAD_REQUEST
                })?;
            
            // 支持 Bearer token 和直接 API key 两种格式
            let key = if provided.starts_with("Bearer ") {
                provided.trim_start_matches("Bearer ").trim()
            } else {
                provided.trim()
            };
            let pbytes = key.as_bytes();
            let ebytes = &**expected;
            
            if constant_time_eq_hash(pbytes, ebytes) {
                return Ok(());
            }
        }
        
        // 开发模式：如果没有提供任何认证头，也允许访问
        if is_dev {
            tracing::debug!("DEV_MODE: Allowing unauthenticated request");
            return Ok(());
        }
        
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

/// 严格认证（向后兼容）
pub async fn authenticate(
    headers: &HeaderMap,
    api_key: &Option<crate::security::SecretVec>,
) -> Result<(), StatusCode> {
    authenticate_with_dev_mode(headers, api_key).await
}
