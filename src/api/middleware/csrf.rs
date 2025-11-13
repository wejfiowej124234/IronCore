//! CSRF防护中间件
//!
//! 提供Cross-Site Request Forgery (CSRF)防护

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use rand::RngCore;
use sha2::{Sha256, Digest};
use tracing::{debug, warn};

/// CSRF Token管理器
#[derive(Clone)]
pub struct CsrfProtection {
    /// 存储有效的CSRF tokens及其过期时间
    tokens: Arc<RwLock<HashMap<String, (String, Instant)>>>,  // token -> (session_id, expiry)
    /// Token有效期
    token_ttl: Duration,
    /// 最大token数量
    max_tokens: usize,
}

impl CsrfProtection {
    /// 创建新的CSRF防护实例
    pub fn new() -> Self {
        Self {
            // ✅ 预分配容量
            tokens: Arc::new(RwLock::new(HashMap::with_capacity(10000))),
            token_ttl: Duration::from_secs(3600),  // 1小时
            max_tokens: 10000,
        }
    }
    
    /// 生成CSRF token
    ///
    /// # Arguments
    /// * `session_id` - 会话ID（用于绑定token到会话）
    ///
    /// # Returns
    /// 32字节十六进制CSRF token
    pub fn generate_token(&self, session_id: &str) -> String {
        // 生成随机token
        let mut token_bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut token_bytes);
        
        // 混入session_id增强绑定
        let mut hasher = Sha256::new();
        hasher.update(&token_bytes);
        hasher.update(session_id.as_bytes());
        hasher.update(&Instant::now().elapsed().as_nanos().to_le_bytes());
        
        let token = format!("{:x}", hasher.finalize());
        
        // 存储token
        let mut tokens = self.tokens.write();
        
        // 清理过期token
        self.cleanup_expired(&mut tokens);
        
        // 如果达到上限，移除最老的
        if tokens.len() >= self.max_tokens {
            if let Some(oldest) = tokens.iter()
                .min_by_key(|(_, (_, time))| time)
                .map(|(k, _)| k.clone())
            {
                tokens.remove(&oldest);
            }
        }
        
        tokens.insert(token.clone(), (session_id.to_string(), Instant::now()));
        
        debug!("Generated CSRF token for session: {}", session_id);
        token
    }
    
    /// validateCSRF token
    ///
    /// # Arguments
    /// * `token` - 要validate的token
    /// * `session_id` - 当前会话ID
    ///
    /// # Returns
    /// true if valid, false otherwise
    pub fn validate_token(&self, token: &str, session_id: &str) -> bool {
        let tokens = self.tokens.read();
        
        if let Some((stored_session, created_at)) = tokens.get(token) {
            // validatesession绑定
            if stored_session != session_id {
                warn!("CSRF token session mismatch");
                return false;
            }
            
            // validate未过期
            if created_at.elapsed() > self.token_ttl {
                warn!("CSRF token expired");
                return false;
            }
            
            debug!("CSRF token validated for session: {}", session_id);
            return true;
        }
        
        warn!("CSRF token not found or invalid");
        false
    }
    
    /// 撤销token（使用后一次性）
    pub fn revoke_token(&self, token: &str) {
        let mut tokens = self.tokens.write();
        tokens.remove(token);
    }
    
    /// 清理过期的tokens
    fn cleanup_expired(&self, tokens: &mut HashMap<String, (String, Instant)>) {
        let now = Instant::now();
        tokens.retain(|_, (_, created_at)| {
            now.duration_since(*created_at) < self.token_ttl
        });
    }
    
    /// 定期清理任务
    pub async fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                let mut tokens = self.tokens.write();
                self.cleanup_expired(&mut tokens);
                debug!("CSRF cleanup: {} tokens remaining", tokens.len());
            }
        });
    }
}

impl Default for CsrfProtection {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF防护中间件
///
/// 仅对状态改变的请求（POST/PUT/DELETE/PATCH）进行validate
pub async fn csrf_protection_middleware(
    csrf: Arc<CsrfProtection>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let headers = req.headers().clone();
    
    // 只对状态改变的请求validateCSRF
    if matches!(method, Method::POST | Method::PUT | Method::DELETE | Method::PATCH) {
        // fromheaderfetchCSRF token
        let csrf_token = headers
            .get("X-CSRF-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        
        // fromheader或cookiefetchsession ID
        let session_id = headers
            .get("X-Session-ID")
            .and_then(|v| v.to_str().ok())
            .or_else(|| {
                // fromAuthorization提取session（简化）
                headers.get("Authorization")
                    .and_then(|v| v.to_str().ok())
            })
            .unwrap_or("anonymous");
        
        // validateCSRF token
        if !csrf.validate_token(csrf_token, session_id) {
            warn!("CSRF validation failed for method: {}", method);
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "CSRF token invalid or missing",
                    "code": "CSRF_VALIDATION_FAILED",
                    "message": "请求被拒绝：缺少有效的CSRF token"
                }))
            ).into_response();
        }
        
        debug!("CSRF validation passed for session: {}", session_id);
    }
    
    // Continue处理请求
    next.run(req).await
}

/// fetchCSRF token的端点helper
pub fn get_csrf_token_response(csrf: Arc<CsrfProtection>, session_id: &str) -> impl IntoResponse {
    let token = csrf.generate_token(session_id);
    
    Json(json!({
        "csrf_token": token,
        "expires_in": 3600,
        "usage": "Include this token in X-CSRF-Token header for POST/PUT/DELETE requests"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_csrf_token_generation() {
        let csrf = CsrfProtection::new();
        let token1 = csrf.generate_token("session1");
        let token2 = csrf.generate_token("session1");
        
        // 每次生成不同的token
        assert_ne!(token1, token2);
        
        // Token应该是64字符（32字节十六进制）
        assert_eq!(token1.len(), 64);
    }
    
    #[test]
    fn test_csrf_token_validation() {
        let csrf = CsrfProtection::new();
        let session_id = "test-session";
        let token = csrf.generate_token(session_id);
        
        // 正确的session应该validate通过
        assert!(csrf.validate_token(&token, session_id));
        
        // error的session应该failed
        assert!(!csrf.validate_token(&token, "wrong-session"));
        
        // error的token应该failed
        assert!(!csrf.validate_token("invalid-token", session_id));
    }
    
    #[test]
    fn test_csrf_token_revocation() {
        let csrf = CsrfProtection::new();
        let session_id = "test-session";
        let token = csrf.generate_token(session_id);
        
        // validate通过
        assert!(csrf.validate_token(&token, session_id));
        
        // 撤销后failed
        csrf.revoke_token(&token);
        assert!(!csrf.validate_token(&token, session_id));
    }
}

