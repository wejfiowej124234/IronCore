//! 存储层trait定义

use async_trait::async_trait;
use crate::auth::{types::User, errors::AuthError};

/// user存储trait
#[async_trait]
pub trait UserStorage: Send + Sync {
    /// 保存user
    async fn save_user(&self, user: &User, password_hash: &str) -> Result<(), AuthError>;
    
    /// 通过Email查找user
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError>;
    
    /// 通过ID查找user
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AuthError>;
    
    /// 通过walletID查找user
    async fn find_by_wallet(&self, wallet_id: &str) -> Result<Option<User>, AuthError>;
    
    /// fetch所有user
    async fn list_users(&self) -> Result<Vec<User>, AuthError>;
    
    /// 更新Last login time
    async fn update_last_login(&self, user_id: &str, timestamp: String) -> Result<(), AuthError>;
    
    /// validatePassword
    async fn verify_password(&self, email: &str, password: &str) -> Result<bool, AuthError>;
    
    /// 更新Password
    async fn update_password(&self, email: &str, new_password_hash: &str) -> Result<(), AuthError>;
    
    /// Deleteuser
    async fn delete_user(&self, user_id: &str) -> Result<(), AuthError>;
    
    /// 关联wallet到user
    async fn link_wallet(&self, user_id: &str, wallet_id: &str) -> Result<(), AuthError>;
}

