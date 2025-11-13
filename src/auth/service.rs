//! 认证服务（整合层）

use std::sync::Arc;
use tracing::info;

use crate::auth::{
    types::*,
    errors::AuthError,
    config::AuthConfig,
    core::{UserService, TokenService, PasswordService},
    storage::UserStorage,
    providers::{OAuthProvider, GoogleProvider},
};

/// 认证服务（门面模式）
pub struct AuthService {
    user_service: Arc<dyn std::any::Any + Send + Sync>,
    token_service: Arc<TokenService>,
    password_service: Arc<PasswordService>,
    google_provider: Arc<GoogleProvider>,
    config: AuthConfig,
}

impl AuthService {
    /// 创建新的认证服务
    ///
    /// # Errors
    /// 返回`AuthError`如果TokenService创建failed
    pub fn new<S: UserStorage + 'static>(
        storage: S,
        config: AuthConfig,
    ) -> Result<Self, AuthError> {
        // 创建各个子服务
        let user_service = Arc::new(UserService::new(
            storage,
            config.clone(),
        ));
        
        let token_service = Arc::new(TokenService::new(
            config.jwt_secret.clone(),
            config.token_expiry,
        )?);  // ✅ 处理Result
        
        let password_service = Arc::new(PasswordService::new(
            config.password.clone(),
        ));
        
        let google_provider = Arc::new(GoogleProvider::new(
            config.oauth.google_client_id.clone(),
        ));
        
        Ok(Self {
            user_service: user_service as Arc<dyn std::any::Any + Send + Sync>,
            token_service,
            password_service,
            google_provider,
            config,
        })
    }
    
    fn get_user_service<S: UserStorage + 'static>(&self) -> &UserService<S> {
        self.user_service
            .downcast_ref::<UserService<S>>()
            .expect("UserService类型不匹配")
    }
    
    /// user注册
    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse, AuthError> {
        info!("业务逻辑: 处理user注册");
        
        // 1. validateEmail格式
        if !Self::is_valid_email(&req.email) {
            return Err(AuthError::InvalidEmail);
        }
        
        // 2. validatePassword一致性（如果提供了confirm_password）
        if let Some(ref confirm) = req.confirm_password {
            if req.password != *confirm {
                return Err(AuthError::PasswordMismatch);
            }
        }
        
        // 3. validatePassword强度
        self.password_service.validate_strength(&req.password)?;
        
        // 4. 哈希Password
        let password_hash = self.password_service.hash_password(&req.password)?;
        
        // 5. 创建user（使用类型擦除后的存储）
        use crate::auth::storage::MemoryStorage;
        let user_service = self.get_user_service::<MemoryStorage>();
        let user = user_service.create_user(req.email, password_hash).await?;
        
        // 6. 生成token
        let access_token = self.token_service.generate_token(&user.id)?;
        let refresh_token = self.token_service.generate_refresh_token(&user.id)?;
        
        Ok(AuthResponse {
            user,
            access_token: access_token.clone(),
            token: Some(access_token), // 兼容前端
            refresh_token: Some(refresh_token),
            token_type: Some("Bearer".to_string()),
            expires_in: Some(self.config.token_expiry),
        })
    }
    
    /// user登录
    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse, AuthError> {
        info!("业务逻辑: 处理user登录");
        
        use crate::auth::storage::MemoryStorage;
        let user_service = self.get_user_service::<MemoryStorage>();
        
        // 1. 查找user（支持Email or wallet ID）
        let user = user_service
            .find_by_email_or_wallet(&req.email)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        
        // 2. validatePassword
        let password_valid = user_service
            .verify_password(&user.email, &req.password)
            .await?;
        
        if !password_valid {
            return Err(AuthError::InvalidCredentials);
        }
        
        // 3. 更新Last login time
        user_service.update_last_login(&user.id).await?;
        
        // 4. 生成token
        let access_token = self.token_service.generate_token(&user.id)?;
        let refresh_token = self.token_service.generate_refresh_token(&user.id)?;
        
        Ok(AuthResponse {
            user,
            access_token: access_token.clone(),
            token: Some(access_token), // 兼容前端
            refresh_token: Some(refresh_token),
            token_type: Some("Bearer".to_string()),
            expires_in: Some(self.config.token_expiry),
        })
    }
    
    /// Google OAuth登录
    pub async fn google_login(&self, id_token: String) -> Result<AuthResponse, AuthError> {
        info!("业务逻辑: 处理Google OAuth登录");
        
        // 1. validateGoogle ID Token
        let oauth_user = self.google_provider.verify_token(&id_token).await?;
        
        use crate::auth::storage::MemoryStorage;
        let user_service = self.get_user_service::<MemoryStorage>();
        
        // 2. 查找或创建user
        let user = if let Some(existing_user) = user_service.find_by_email(&oauth_user.email).await? {
            // 已存在user，更新登录时间
            user_service.update_last_login(&existing_user.id).await?;
            existing_user
        } else {
            // 新user，创建账户
            let password_hash = "oauth_user_no_password".to_string();
            user_service.create_user(oauth_user.email, password_hash).await?
        };
        
        // 3. 生成token
        let access_token = self.token_service.generate_token(&user.id)?;
        let refresh_token = self.token_service.generate_refresh_token(&user.id)?;
        
        Ok(AuthResponse {
            user,
            access_token: access_token.clone(),
            token: Some(access_token), // 兼容前端
            refresh_token: Some(refresh_token),
            token_type: Some("Bearer".to_string()),
            expires_in: Some(self.config.token_expiry),
        })
    }
    
    /// 通过Tokenfetchuser
    pub async fn get_user_by_token(&self, token: &str) -> Result<User, AuthError> {
        // 1. validatetoken
        let user_id = self.token_service.verify_token(token)?;
        
        use crate::auth::storage::MemoryStorage;
        let user_service = self.get_user_service::<MemoryStorage>();
        
        // 2. 查找user
        user_service
            .find_by_id(&user_id)
            .await?
            .ok_or(AuthError::UserNotFound)
    }
    
    /// 修改Password
    pub async fn change_password(
        &self,
        token: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AuthError> {
        // 1. fetchuser
        let user = self.get_user_by_token(token).await?;
        
        use crate::auth::storage::MemoryStorage;
        let user_service = self.get_user_service::<MemoryStorage>();
        
        // 2. validate旧Password
        let password_valid = user_service.verify_password(&user.email, old_password).await?;
        if !password_valid {
            return Err(AuthError::InvalidCredentials);
        }
        
        // 3. validate新Password强度
        self.password_service.validate_strength(new_password)?;
        
        // 4. 哈希新Password
        let new_hash = self.password_service.hash_password(new_password)?;
        
        // 5. 更新Password
        user_service.update_password(&user.email, &new_hash).await?;
        
        Ok(())
    }
    
    /// check名称可用性
    pub async fn check_name_available(&self, name: &str) -> Result<NameCheckResponse, AuthError> {
        // validate名称格式
        if name.len() < 3 || name.len() > 32 {
            return Ok(NameCheckResponse {
                name: name.to_string(),
                available: false,
                message: "名称长度应在 3-32 个字符之间".to_string(),
            });
        }
        
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Ok(NameCheckResponse {
                name: name.to_string(),
                available: false,
                message: "名称只能包含字母、数字、下划线和连字符".to_string(),
            });
        }
        
        if name.chars().next().map(|c| c.is_numeric()).unwrap_or(false) {
            return Ok(NameCheckResponse {
                name: name.to_string(),
                available: false,
                message: "名称不能以数字开头".to_string(),
            });
        }
        
        // check是否已被使用（这里简化为总是可用）
        Ok(NameCheckResponse {
            name: name.to_string(),
            available: true,
            message: "名称可用".to_string(),
        })
    }
    
    // 辅助函数
    fn is_valid_email(email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }
}

