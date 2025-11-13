//! Token管理服务

use crate::auth::errors::AuthError;
use tracing::info;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
use serde::{Serialize, Deserialize};

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// User ID
    sub: String,
    /// 过期时间戳
    exp: usize,
    /// 签发时间戳
    iat: usize,
}

/// Token管理服务
pub struct TokenService {
    /// JWT密钥（使用Zeroizing保护内存）
    secret: zeroize::Zeroizing<String>,
    
    /// TokenExpiration time (seconds)
    expiry: u64,
}

impl TokenService {
    /// 创建新的Token服务（带密钥强度validate）
    ///
    /// # Security
    /// - 拒绝弱密钥（<32字符）
    /// - 拒绝已知的测试密钥
    /// - 使用Zeroizing保护内存
    pub fn new(secret: String, expiry: u64) -> Result<Self, AuthError> {
        // validate密钥强度
        if secret.len() < 32 {
            return Err(AuthError::InvalidInput(
                "JWT secret must be at least 32 characters".to_string()
            ));
        }
        
        // 拒绝已知的弱密钥
        const WEAK_SECRETS: &[&str] = &[
            "dev_secret_key",
            "test",
            "secret",
            "password",
            "12345678",
            "admin",
            "default",
        ];
        
        for weak in WEAK_SECRETS {
            if secret.to_lowercase().contains(weak) {
                return Err(AuthError::InvalidInput(
                    format!("Weak or common JWT secret detected: contains '{}'", weak)
                ));
            }
        }
        
        Ok(Self {
            secret: zeroize::Zeroizing::new(secret),
            expiry,
        })
    }
    
    /// 生成Access Token（使用真正的JWT）
    pub fn generate_token(&self, user_id: &str) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let expiration = now + self.expiry as usize;
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
            iat: now,
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes())
        ).map_err(|e| AuthError::InternalError(format!("JWT生成failed: {}", e)))?;
        
        // ✅ 修复：不记录user_id到日志
        info!("生成真正的JWT令牌");
        // 为了配合测试与下游使用习惯，添加显式前缀
        Ok(format!("token_{}", token))
    }
    
    /// validateToken并返回User ID（使用真正的JWTvalidate）
    pub fn verify_token(&self, token: &str) -> Result<String, AuthError> {
        let validation = Validation::new(Algorithm::HS256);
        // 兼容带有前缀的token（例如 token_ 或 refresh_）
        let raw = if let Some(stripped) = token.strip_prefix("token_") {
            stripped
        } else if let Some(stripped) = token.strip_prefix("refresh_") {
            stripped
        } else {
            token
        };
        
        let token_data = decode::<Claims>(
            raw,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation
        ).map_err(|_| AuthError::InvalidToken)?;
        
        Ok(token_data.claims.sub)
    }
    
    /// 生成Refresh Token（7天有效期）
    pub fn generate_refresh_token(&self, user_id: &str) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let expiration = now + (7 * 24 * 60 * 60); // 7天
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
            iat: now,
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes())
        ).map_err(|e| AuthError::InternalError(format!("Refresh token生成failed: {}", e)))?;
        
        // ✅ 修复：不记录user_id到日志
        info!("生成真正的Refresh token");
        // 为了配合测试与下游使用习惯，添加显式前缀
        Ok(format!("refresh_{}", token))
    }
    
    /// validateRefresh Token并返回User ID
    pub fn verify_refresh_token(&self, token: &str) -> Result<String, AuthError> {
        // Refresh token和access token使用相同的validate逻辑
        self.verify_token(token)
    }
}

// ❌ REMOVED: Default实现已Delete，防止使用弱密钥
// 
// 之前的Default会使用"dev_secret_key"，这是严重的安全漏洞！
// 现在必须显式提供强密钥，不允许使用默认值。
//
// impl Default for TokenService {
//     fn default() -> Self {
//         Self::new("dev_secret_key".to_string(), 3600)  // 🔴 危险！
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        // ✅ 使用强密钥（避免弱模式）
        let service = TokenService::new(
            "mY$uP3r$tr0nG_jWt_k3Y_f0r_t3sting!@#".to_string(),
            3600
        ).expect("Failed to create TokenService");
        
        let token = service.generate_token("user-123").unwrap();
        assert!(token.starts_with("token_"));
    }

    #[test]
    fn test_token_verification() {
        // ✅ 使用强密钥
        let service = TokenService::new(
            "mY$uP3r$tr0nG_jWt_k3Y_f0r_t3sting!@#".to_string(),
            3600
        ).expect("Failed to create TokenService");
        
        let token = service.generate_token("user-123").unwrap();
        let result = service.verify_token(&token);
        assert!(result.is_ok());
        
        // 无效token
        let invalid = service.verify_token("invalid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_refresh_token() {
        // ✅ 使用强密钥
        let service = TokenService::new(
            "mY$uP3r$tr0nG_jWt_k3Y_f0r_t3sting!@#".to_string(),
            3600
        ).expect("Failed to create TokenService");
        
        let refresh = service.generate_refresh_token("user-123").unwrap();
        assert!(refresh.starts_with("refresh_"));
        
        let result = service.verify_refresh_token(&refresh);
        assert!(result.is_ok());
    }
}

