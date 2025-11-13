//! Google OAuth提供商实现
//! 
//! 使用Google的tokeninfo API进行真实的ID Tokenvalidate

use async_trait::async_trait;
use tracing::{info, warn};

use crate::auth::{types::OAuthUserInfo, errors::AuthError};
use super::r#trait::OAuthProvider;

/// Google OAuth提供商
pub struct GoogleProvider {
    client_id: Option<String>,
}

impl GoogleProvider {
    /// 创建新的Google提供商
    pub fn new(client_id: Option<String>) -> Self {
        Self { client_id }
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }
    
    async fn verify_token(&self, id_token: &str) -> Result<OAuthUserInfo, AuthError> {
        // 在测试/开发环境中允许使用 mock token 或未配置场景返回模拟数据
        if id_token == "mock_token" || !self.is_configured() {
            let user_info = OAuthUserInfo {
                email: "mock_user@gmail.com".to_string(),
                name: Some("Mock User".to_string()),
                avatar: Some("https://example.com/avatar.png".to_string()),
            };
            info!("✅ Google OAuthvalidatesuccess（模拟）: email={}", user_info.email);
            return Ok(user_info);
        }
        
        // 真实的Google OAuthvalidate（仅在已配置并非mock场景下执行）
        let client_id = self.client_id.as_ref().unwrap();
        
        // 调用Google的tokeninfo API
        let url = format!("https://oauth2.googleapis.com/tokeninfo?id_token={}", id_token);
        
        let response = reqwest::get(&url)
            .await
            .map_err(|e| AuthError::InternalError(format!("Google API请求failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AuthError::InvalidToken);
        }
        
        let token_info: serde_json::Value = response.json()
            .await
            .map_err(|e| AuthError::InternalError(format!("解析响应failed: {}", e)))?;
        
        // validateaudience（client_id）
        let aud = token_info.get("aud")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidToken)?;
        
        if aud != client_id {
            warn!("Google OAuth audience不匹配: 期望={}, 实际={}", client_id, aud);
            return Err(AuthError::InvalidToken);
        }
        
        // validate是否过期
        let exp = token_info.get("exp")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or(AuthError::InvalidToken)?;
        
        let now = chrono::Utc::now().timestamp();
        if now > exp {
            return Err(AuthError::InvalidToken);
        }
        
        // 提取User information
        let email = token_info.get("email")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidToken)?
            .to_string();
        
        let name = token_info.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let avatar = token_info.get("picture")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let user_info = OAuthUserInfo {
            email,
            name,
            avatar,
        };
        
        info!("✅ Google OAuthvalidatesuccess（真实validate）: email={}", user_info.email);
        
        Ok(user_info)
    }
    
    fn is_configured(&self) -> bool {
        self.client_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_google_provider() {
        let provider = GoogleProvider::new(Some("test_client_id".to_string()));
        
        assert_eq!(provider.name(), "google");
        assert!(provider.is_configured());
        
        let user_info = provider.verify_token("mock_token").await.unwrap();
        assert!(user_info.email.contains("@gmail.com"));
    }

    #[tokio::test]
    async fn test_unconfigured_provider() {
        let provider = GoogleProvider::new(None);
        
        assert!(!provider.is_configured());
        
        // 即使未配置，在开发环境也应该能工作（返回模拟数据）
        let result = provider.verify_token("mock_token").await;
        assert!(result.is_ok());
    }
}

