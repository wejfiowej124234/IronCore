//! 内存存储实现（用于开发和测试）

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::auth::{types::User, errors::AuthError};
use super::r#trait::UserStorage;

/// 内存存储
pub struct MemoryStorage {
    /// user列表（email -> User）
    users: Arc<RwLock<HashMap<String, User>>>,
    
    /// Password哈希（email -> hash）
    passwords: Arc<RwLock<HashMap<String, String>>>,
    
    /// User ID索引（id -> email）
    user_ids: Arc<RwLock<HashMap<String, String>>>,
    
    /// wallet索引（wallet_id -> user_id）
    wallet_index: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    /// 创建新的内存存储
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            passwords: Arc::new(RwLock::new(HashMap::new())),
            user_ids: Arc::new(RwLock::new(HashMap::new())),
            wallet_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl UserStorage for MemoryStorage {
    async fn save_user(&self, user: &User, password_hash: &str) -> Result<(), AuthError> {
        let mut users = self.users.write().await;
        let mut passwords = self.passwords.write().await;
        let mut user_ids = self.user_ids.write().await;
        
        // 保存user
        users.insert(user.email.clone(), user.clone());
        
        // 保存Password哈希
        passwords.insert(user.email.clone(), password_hash.to_string());
        
        // 保存ID索引
        user_ids.insert(user.id.clone(), user.email.clone());
        
        info!("user已保存到内存存储: email={}", user.email);
        
        Ok(())
    }
    
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        let users = self.users.read().await;
        Ok(users.get(email).cloned())
    }
    
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AuthError> {
        let user_ids = self.user_ids.read().await;
        if let Some(email) = user_ids.get(id) {
            let email = email.clone();
            drop(user_ids);
            return self.find_by_email(&email).await;
        }
        Ok(None)
    }
    
    async fn find_by_wallet(&self, wallet_id: &str) -> Result<Option<User>, AuthError> {
        let wallet_index = self.wallet_index.read().await;
        if let Some(user_id) = wallet_index.get(wallet_id) {
            let user_id = user_id.clone();
            drop(wallet_index);
            return self.find_by_id(&user_id).await;
        }
        Ok(None)
    }
    
    async fn list_users(&self) -> Result<Vec<User>, AuthError> {
        let users = self.users.read().await;
        Ok(users.values().cloned().collect())
    }
    
    async fn update_last_login(&self, user_id: &str, timestamp: String) -> Result<(), AuthError> {
        let user_ids = self.user_ids.read().await;
        if let Some(email) = user_ids.get(user_id) {
            let email = email.clone();
            drop(user_ids);
            
            let mut users = self.users.write().await;
            if let Some(user) = users.get_mut(&email) {
                user.last_login = Some(timestamp);
                info!("更新userLast login time: user_id={}", user_id);
                return Ok(());
            }
        }
        Err(AuthError::UserNotFound)
    }
    
    async fn verify_password(&self, email: &str, password: &str) -> Result<bool, AuthError> {
        let passwords = self.passwords.read().await;
        if let Some(stored_hash) = passwords.get(email) {
            // 使用bcrypt进行安全的Passwordvalidate（常量时间比较）
            Ok(bcrypt::verify(password, stored_hash).unwrap_or(false))
        } else {
            Ok(false)
        }
    }
    
    async fn update_password(&self, email: &str, new_password_hash: &str) -> Result<(), AuthError> {
        let mut passwords = self.passwords.write().await;
        if passwords.contains_key(email) {
            passwords.insert(email.to_string(), new_password_hash.to_string());
            info!("更新userPassword: email={}", email);
            Ok(())
        } else {
            Err(AuthError::UserNotFound)
        }
    }
    
    async fn delete_user(&self, user_id: &str) -> Result<(), AuthError> {
        let user_ids = self.user_ids.read().await;
        if let Some(email) = user_ids.get(user_id).cloned() {
            drop(user_ids);
            
            let mut users = self.users.write().await;
            let mut passwords = self.passwords.write().await;
            let mut user_ids_mut = self.user_ids.write().await;
            
            users.remove(&email);
            passwords.remove(&email);
            user_ids_mut.remove(user_id);
            
            info!("Deleteuser: user_id={}", user_id);
            Ok(())
        } else {
            Err(AuthError::UserNotFound)
        }
    }
    
    async fn link_wallet(&self, user_id: &str, wallet_id: &str) -> Result<(), AuthError> {
        let user_ids = self.user_ids.read().await;
        if let Some(email) = user_ids.get(user_id).cloned() {
            drop(user_ids);
            
            let mut users = self.users.write().await;
            if let Some(user) = users.get_mut(&email) {
                if !user.wallets.contains(&wallet_id.to_string()) {
                    user.wallets.push(wallet_id.to_string());
                    
                    // 更新wallet索引
                    let mut wallet_index = self.wallet_index.write().await;
                    wallet_index.insert(wallet_id.to_string(), user_id.to_string());
                    
                    info!("关联wallet到user: user_id={}, wallet_id={}", user_id, wallet_id);
                }
                return Ok(());
            }
        }
        Err(AuthError::UserNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::types::UserStatus;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new();
        
        let user = User {
            id: "user-123".to_string(),
            email: "test@example.com".to_string(),
            username: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_login: None,
            wallets: vec![],
            roles: Some(vec!["user".to_string()]),
            status: Some(UserStatus::Active),
        };
        
        // 保存user
        storage.save_user(&user, "hashed_password").await.unwrap();
        
        // 查找user
        let found = storage.find_by_email("test@example.com").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "user-123");
        
        // 通过ID查找
        let found_by_id = storage.find_by_id("user-123").await.unwrap();
        assert!(found_by_id.is_some());
    }
}

