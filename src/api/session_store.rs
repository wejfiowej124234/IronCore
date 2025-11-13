//! 会话存储 - 管理user会话和wallet密钥
//!
//! 功能：
//! - Token到User ID的映射
//! - userwalletPassword的会话缓存
//! - 会话过期管理

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use anyhow::{Result, bail};

/// Token信息
#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub user_id: String,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
}

/// walletPassword缓存
#[derive(Clone)]
pub struct WalletPasswordCache {
    /// wallet_name -> password
    passwords: HashMap<String, String>,
    /// 最后更新时间
    last_updated: SystemTime,
}

/// 会话存储
#[derive(Clone)]
pub struct SessionStore {
    /// token -> TokenInfo
    tokens: Arc<RwLock<HashMap<String, TokenInfo>>>,
    /// user_id -> WalletPasswordCache
    wallet_passwords: Arc<RwLock<HashMap<String, WalletPasswordCache>>>,
}

impl SessionStore {
    /// 创建新的会话存储
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            wallet_passwords: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册token（user登录时调用）
    pub async fn register_token(&self, token: &str, user_id: &str, ttl_seconds: u64) {
        let now = SystemTime::now();
        let token_info = TokenInfo {
            user_id: user_id.to_string(),
            created_at: now,
            expires_at: now + Duration::from_secs(ttl_seconds),
        };
        
        let mut tokens = self.tokens.write().await;
        tokens.insert(token.to_string(), token_info);
        
        tracing::debug!("✅ Token已注册: user_id={}, ttl={}秒", user_id, ttl_seconds);
    }
    
    /// validatetoken并返回User ID
    pub async fn validate_token(&self, token: &str) -> Result<String> {
        let tokens = self.tokens.read().await;
        
        match tokens.get(token) {
            Some(info) => {
                // check是否过期
                let now = SystemTime::now();
                if now > info.expires_at {
                    bail!("Token已过期");
                }
                Ok(info.user_id.clone())
            }
            None => bail!("无效的Token"),
        }
    }
    
    /// 撤销token（user登出时调用）
    pub async fn revoke_token(&self, token: &str) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        if tokens.remove(token).is_some() {
            tracing::info!("✅ Token已撤销");
            Ok(())
        } else {
            bail!("Token不存在");
        }
    }
    
    /// 保存walletPassword到会话
    pub async fn cache_wallet_password(&self, user_id: &str, wallet_name: &str, password: &str) {
        let mut passwords = self.wallet_passwords.write().await;
        
        let cache = passwords.entry(user_id.to_string())
            .or_insert_with(|| WalletPasswordCache {
                passwords: HashMap::new(),
                last_updated: SystemTime::now(),
            });
        
        cache.passwords.insert(wallet_name.to_string(), password.to_string());
        cache.last_updated = SystemTime::now();
        
        tracing::debug!("✅ walletPassword已缓存到会话: user={}, wallet={}", user_id, wallet_name);
    }
    
    /// from会话fetchwalletPassword
    pub async fn get_wallet_password(&self, user_id: &str, wallet_name: &str) -> Option<String> {
        let passwords = self.wallet_passwords.read().await;
        passwords.get(user_id)?.passwords.get(wallet_name).cloned()
    }
    
    /// Clearuser的所有会话数据（登出时调用）
    pub async fn clear_user_session(&self, user_id: &str) {
        let mut passwords = self.wallet_passwords.write().await;
        passwords.remove(user_id);
        
        tracing::info!("✅ user会话已Clear: user_id={}", user_id);
    }
    
    /// 清理过期的token（定期任务）
    pub async fn cleanup_expired_tokens(&self) -> usize {
        let mut tokens = self.tokens.write().await;
        let now = SystemTime::now();
        let initial_count = tokens.len();
        
        tokens.retain(|_, info| now <= info.expires_at);
        
        let removed = initial_count - tokens.len();
        if removed > 0 {
            tracing::info!("✅ 清理了 {} 个过期token", removed);
        }
        removed
    }
    
    /// fetch所有活跃的user会话数量
    pub async fn get_active_sessions_count(&self) -> usize {
        let tokens = self.tokens.read().await;
        tokens.len()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_token_lifecycle() {
        let store = SessionStore::new();
        
        // 注册token
        store.register_token("test_token", "user123", 60).await;
        
        // validatetoken
        let user_id = store.validate_token("test_token").await.unwrap();
        assert_eq!(user_id, "user123");
        
        // 撤销token
        store.revoke_token("test_token").await.unwrap();
        
        // validatefailed
        assert!(store.validate_token("test_token").await.is_err());
    }
    
    #[tokio::test]
    async fn test_token_expiration() {
        let store = SessionStore::new();
        
        // 注册1秒后过期的token
        store.register_token("short_token", "user123", 1).await;
        
        // 立即validate应该success
        assert!(store.validate_token("short_token").await.is_ok());
        
        // 等待2秒
        sleep(Duration::from_secs(2)).await;
        
        // validate应该failed（过期）
        assert!(store.validate_token("short_token").await.is_err());
    }
    
    #[tokio::test]
    async fn test_wallet_password_cache() {
        let store = SessionStore::new();
        
        // 缓存walletPassword
        store.cache_wallet_password("user1", "wallet1", "pass123").await;
        store.cache_wallet_password("user1", "wallet2", "pass456").await;
        store.cache_wallet_password("user2", "wallet3", "pass789").await;
        
        // fetchwalletPassword
        assert_eq!(
            store.get_wallet_password("user1", "wallet1").await,
            Some("pass123".to_string())
        );
        assert_eq!(
            store.get_wallet_password("user1", "wallet2").await,
            Some("pass456".to_string())
        );
        assert_eq!(
            store.get_wallet_password("user2", "wallet3").await,
            Some("pass789".to_string())
        );
        
        // user1不能访问user2的wallet
        assert_eq!(
            store.get_wallet_password("user1", "wallet3").await,
            None
        );
        
        // Clearuser1的会话
        store.clear_user_session("user1").await;
        assert_eq!(store.get_wallet_password("user1", "wallet1").await, None);
        
        // user2的会话不受影响
        assert_eq!(
            store.get_wallet_password("user2", "wallet3").await,
            Some("pass789".to_string())
        );
    }
}

