//! OAuth提供商trait定义

use async_trait::async_trait;
use crate::auth::{types::OAuthUserInfo, errors::AuthError};

/// OAuth提供商trait
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// 提供商名称
    fn name(&self) -> &str;
    
    /// validateID Token并返回User information
    async fn verify_token(&self, id_token: &str) -> Result<OAuthUserInfo, AuthError>;
    
    /// 是否已配置
    fn is_configured(&self) -> bool;
}

