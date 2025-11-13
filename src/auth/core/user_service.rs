//! user管理服务

use crate::auth::{
    types::*,
    errors::AuthError,
    storage::UserStorage,
    config::AuthConfig,
};
use tracing::info;
use uuid::Uuid;

/// user管理服务
pub struct UserService<S: UserStorage> {
    storage: S,
    #[allow(dead_code)]
    config: AuthConfig,
}

impl<S: UserStorage> UserService<S> {
    /// 创建新的user服务
    pub fn new(storage: S, config: AuthConfig) -> Self {
        Self { storage, config }
    }
    
    /// 创建user
    pub async fn create_user(
        &self,
        email: String,
        password_hash: String,
    ) -> Result<User, AuthError> {
        // checkEmail是否已存在
        if self.storage.find_by_email(&email).await?.is_some() {
            return Err(AuthError::EmailExists);
        }
        
        // 创建user
        let user = User {
            id: Uuid::new_v4().to_string(),
            email: email.clone(),
            username: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_login: None,
            wallets: Vec::new(),
            roles: Some(vec!["user".to_string()]),
            status: Some(UserStatus::Active),
        };
        
        // 保存user
        self.storage.save_user(&user, &password_hash).await?;
        
        info!("user创建success: id={}, email={}", user.id, email);
        
        Ok(user)
    }
    
    /// 通过Email查找user
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        self.storage.find_by_email(email).await
    }
    
    /// 通过ID查找user
    pub async fn find_by_id(&self, id: &str) -> Result<Option<User>, AuthError> {
        self.storage.find_by_id(id).await
    }
    
    /// 通过walletID查找user
    pub async fn find_by_wallet(&self, wallet_id: &str) -> Result<Option<User>, AuthError> {
        self.storage.find_by_wallet(wallet_id).await
    }
    
    /// 通过Email or wallet ID查找user
    pub async fn find_by_email_or_wallet(&self, identifier: &str) -> Result<Option<User>, AuthError> {
        // 先尝试Email
        if let Some(user) = self.find_by_email(identifier).await? {
            return Ok(Some(user));
        }
        
        // 再尝试walletID
        self.find_by_wallet(identifier).await
    }
    
    /// 更新Last login time
    pub async fn update_last_login(&self, user_id: &str) -> Result<(), AuthError> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        self.storage.update_last_login(user_id, timestamp).await
    }
    
    /// validatePassword
    pub async fn verify_password(&self, email: &str, password: &str) -> Result<bool, AuthError> {
        self.storage.verify_password(email, password).await
    }
    
    /// 更新Password
    pub async fn update_password(&self, email: &str, new_password_hash: &str) -> Result<(), AuthError> {
        self.storage.update_password(email, new_password_hash).await
    }
    
    /// 关联wallet到user
    pub async fn link_wallet(&self, user_id: &str, wallet_id: &str) -> Result<(), AuthError> {
        self.storage.link_wallet(user_id, wallet_id).await
    }
    
    /// fetchuser列表
    pub async fn list_users(&self) -> Result<Vec<User>, AuthError> {
        self.storage.list_users().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::storage::MemoryStorage;

    #[tokio::test]
    async fn test_create_user() {
        let storage = MemoryStorage::new();
        let service = UserService::new(storage, AuthConfig::default());
        
        let user = service.create_user(
            "test@example.com".to_string(),
            "hashed_password".to_string()
        ).await.unwrap();
        
        assert_eq!(user.email, "test@example.com");
        assert!(user.status == Some(UserStatus::Active));
    }

    #[tokio::test]
    async fn test_duplicate_email() {
        let storage = MemoryStorage::new();
        let service = UserService::new(storage, AuthConfig::default());
        
        // 第一次创建success
        service.create_user(
            "test@example.com".to_string(),
            "hash1".to_string()
        ).await.unwrap();
        
        // 第二次应该failed
        let result = service.create_user(
            "test@example.com".to_string(),
            "hash2".to_string()
        ).await;
        
        assert!(result.is_err());
    }
}

