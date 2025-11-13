//! User authentication API routes
//!
//! Provides registration, login, password management, etc.
//! Uses separate users.db database to store user information

use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

// Use new user database module
use super::user_db::{UserDatabase, CreateUserRequest as DbCreateUserRequest, LoginRequest as DbLoginRequest};

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// Email
    pub email: String,
    /// Username (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Creation time
    pub created_at: String,
    /// Last login time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<String>,
    /// Associated wallet list
    pub wallets: Vec<String>,
}

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Email
    pub email: String,
    /// Password
    pub password: String,
    /// Confirm password
    pub confirm_password: String,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Email or wallet ID
    pub email_or_wallet_id: String,
    /// Password
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    /// Access token
    pub access_token: String,
    /// Token (compatibility field, same as access_token)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Refresh token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// User information
    pub user: User,
    /// Token type
    pub token_type: String,
    /// Expiration time (seconds)
    pub expires_in: u64,
}

/// Google OAuth request
#[derive(Debug, Deserialize)]
pub struct GoogleAuthRequest {
    /// Google ID Token
    pub id_token: String,
}

/// API state (uses separate database)
#[derive(Clone)]
pub struct AuthApiState {
    /// User database connection pool
    pub user_db: Arc<UserDatabase>,
}

impl AuthApiState {
    /// Create new API state
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let user_db = UserDatabase::new(database_url).await?;
        
        // Ensure Demo user exists
        user_db.ensure_demo_user().await?;
        
        Ok(Self {
            user_db: Arc::new(user_db),
        })
    }
}

/// 创建认证路由
pub fn create_auth_routes(state: AuthApiState) -> Router {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/google", post(google_auth))
        .route("/api/auth/me", get(get_current_user))
        .route("/api/auth/change-password", post(change_password))
        .route("/api/auth/refresh", post(refresh_token))
        .route("/api/auth/logout", post(logout))
        .with_state(state)
}

/// POST /api/auth/register
/// 
/// user注册（使用独立数据库）
async fn register(
    State(state): State<AuthApiState>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    info!("收到Registration request: email={}", req.email);
    
    // validate两次Password是否一致
    if req.password != req.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Password mismatch",
                "message": "两次输入的Password不一致"
            }))
        ).into_response();
    }
    
    // 使用数据库创建user
    let db_req = DbCreateUserRequest {
        email: req.email.clone(),
        password: req.password,
        username: None,
    };
    
    match state.user_db.create_user(db_req).await {
        Ok(user) => {
            info!("user注册success: id={}, email={}", user.id, user.email);
            
            // 生成Access token
            let access_token = generate_token(&user.id);
            
            // 转换为API User格式
            let api_user = User {
                id: user.id,
                email: user.email,
                username: user.username,
                created_at: user.created_at.to_rfc3339(),
                last_login: user.last_login_at.map(|dt| dt.to_rfc3339()),
                wallets: Vec::new(),
            };
            
            Json(AuthResponse {
                access_token: access_token.clone(),
                token: Some(access_token),
                refresh_token: None,
                user: api_user,
                token_type: "Bearer".to_string(),
                expires_in: 3600,
            }).into_response()
        }
        Err(e) => {
            let error_msg = e.to_string();
            let (status, error_type, message) = if error_msg.contains("UNIQUE constraint failed") {
                (StatusCode::CONFLICT, "Email exists", "该Email已被注册")
            } else if error_msg.contains("Invalid email") {
                (StatusCode::BAD_REQUEST, "Invalid email", "Email格式不正确")
            } else if error_msg.contains("at least 8") {
                (StatusCode::BAD_REQUEST, "Weak password", "Password长度至少8位")
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Registration failed", "注册failed，请稍后重试")
            };
            
            (
                status,
                Json(serde_json::json!({
                    "error": error_type,
                    "message": message
                }))
            ).into_response()
        }
    }
}

/// POST /api/auth/login
/// 
/// user登录（使用独立数据库）
async fn login(
    State(state): State<AuthApiState>,
    Json(req): Json<LoginRequest>,
) -> Response {
    info!("收到Login request: identifier={}", req.email_or_wallet_id);
    
    // 使用数据库validate登录
    let db_req = DbLoginRequest {
        email: req.email_or_wallet_id.clone(),
        password: req.password,
    };
    
    match state.user_db.verify_login(db_req).await {
        Ok(user) => {
            info!("user登录success: id={}, email={}", user.id, user.email);
            
            // 生成Access token
            let access_token = generate_token(&user.id);
            
            // fetchuser的wallet列表
            let wallets = state.user_db.get_user_wallets(&user.id).await.unwrap_or_default();
            
            // 转换为API User格式
            let api_user = User {
                id: user.id,
                email: user.email,
                username: user.username,
                created_at: user.created_at.to_rfc3339(),
                last_login: user.last_login_at.map(|dt| dt.to_rfc3339()),
                wallets,
            };
            
            Json(AuthResponse {
                access_token: access_token.clone(),
                token: Some(access_token),
                refresh_token: None,
                user: api_user,
                token_type: "Bearer".to_string(),
                expires_in: 3600,
            }).into_response()
        }
        Err(e) => {
            let error_msg = e.to_string();
            let message = if error_msg.contains("locked") {
                "账户已锁定，请稍后再试"
            } else if error_msg.contains("disabled") {
                "账户已被禁用"
            } else {
                "Email或Passworderror"
            };
            
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid credentials",
                    "message": message
                }))
            ).into_response()
        }
    }
}

/// POST /api/auth/google
/// 
/// Google OAuth 登录/注册
async fn google_auth(
    State(state): State<AuthApiState>,
    Json(_req): Json<GoogleAuthRequest>,
) -> Response {
    info!("收到 Google OAuth 请求");
    
    // 简化实现：解析 ID Token（实际应validatesign）
    // 这里仅做演示，实际应使用 Google API validate
    
    let email = format!("google_user_{}@gmail.com", &Uuid::new_v4().to_string()[..8]);
    
    // 查找或创建user
    let mut users = state.users.lock().await;
    let existing_user = users.iter().find(|u| u.email == email);
    
    let user = if let Some(existing) = existing_user {
        // 已存在user，更新登录时间
        let mut user = existing.clone();
        user.last_login = Some(chrono::Utc::now().to_rfc3339());
        user
    } else {
        // 新user，创建账户
        let user_id = Uuid::new_v4().to_string();
        let user = User {
            id: user_id,
            email: email.clone(),
            username: Some("Google User".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_login: Some(chrono::Utc::now().to_rfc3339()),
            wallets: Vec::new(),
        };
        users.push(user.clone());
        info!("通过 Google 创建新user: email={}", email);
        user
    };
    
    drop(users);
    
    // 生成Access token
    let access_token = generate_token(&user.id);
    
    // 保存token映射（用于后续validate）
    state.tokens.lock().await.insert(access_token.clone(), user.id.clone());
    
    Json(AuthResponse {
        access_token: access_token.clone(),
        token: Some(access_token), // 兼容字段
        refresh_token: None,
        user,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
    }).into_response()
}

/// GET /api/auth/me
/// 
/// fetch当前User information
async fn get_current_user(
    State(state): State<AuthApiState>,
    headers: HeaderMap,
) -> Response {
    // from Authorization 头提取令牌
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized",
                    "code": "UNAUTHORIZED",
                    "message": "未提供Access token"
                }))
            ).into_response();
        }
    };
    
    // from token 映射中查找User ID
    let tokens = state.tokens.lock().await;
    let user_id = tokens.get(&token);
    
    let user_id = match user_id {
        Some(id) => id.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid token",
                    "code": "INVALID_TOKEN",
                    "message": "无效的Access token或令牌已过期"
                }))
            ).into_response();
        }
    };
    
    drop(tokens); // 释放锁
    
    // 查找user
    let users = state.users.lock().await;
    let user = users.iter().find(|u| u.id == user_id);
    
    // ✅ 安全的user查找，避免unwrap
    // 前端需要格式：{ user: { id, email, ... } }
    match user {
        Some(u) => Json(serde_json::json!({
            "user": u.clone()
        })).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "User not found",
                "code": "USER_NOT_FOUND",
                "message": "user不存在"
            }))
        ).into_response()
    }
}

/// POST /api/auth/change-password
/// 
/// 修改Password
async fn change_password(
    State(state): State<AuthApiState>,
    headers: HeaderMap,
    Json(req): Json<serde_json::Value>,
) -> Response {
    // validate令牌
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized",
                    "code": "UNAUTHORIZED",
                    "message": "未提供Access token"
                }))
            ).into_response();
        }
    };
    
    // from token 映射中查找User ID
    let tokens = state.tokens.lock().await;
    let user_id = match tokens.get(&token) {
        Some(id) => id.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid token",
                    "code": "INVALID_TOKEN",
                    "message": "无效的Access token"
                }))
            ).into_response();
        }
    };
    drop(tokens);
    
    // 提取Password（支持两种字段名）
    let old_password = req.get("current_password")
        .or_else(|| req.get("old_password"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let new_password = req.get("new_password").and_then(|v| v.as_str()).unwrap_or("");
    
    if new_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Weak password",
                "message": "新Password长度至少8位"
            }))
        ).into_response();
    }
    
    // 查找user
    let users = state.users.lock().await;
    let user = users.iter().find(|u| u.id == user_id);
    
    // ✅ 安全的user查找，避免unwrap
    let user = match user {
        Some(u) => u,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "User not found",
                    "message": "user不存在"
                }))
            ).into_response();
        }
    };
    
    // validate旧Password
    let mut passwords = state.passwords.lock().await;
    let stored_hash = passwords.get(&user.email);
    
    // ✅ 安全的Passwordvalidate，避免unwrap
    let password_valid = match stored_hash {
        Some(hash) => verify_password(old_password, hash),
        None => false,
    };
    
    if !password_valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid password",
                "message": "原Passworderror"
            }))
        ).into_response();
    }
    
    // 更新Password
    let new_hash = hash_password(new_password);
    passwords.insert(user.email.clone(), new_hash);
    
    info!("user修改Passwordsuccess: email={}", user.email);
    
    Json(serde_json::json!({
        "success": true,
        "message": "Password修改success"
    })).into_response()
}

/// POST /api/auth/refresh
/// 
/// 刷新Access token
#[derive(Debug, Deserialize)]
struct RefreshTokenRequest {
    refresh_token: String,
}

async fn refresh_token(
    State(_state): State<AuthApiState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Response {
    // validate refresh token
    if req.refresh_token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "code": "MISSING_REFRESH_TOKEN",
                "message": "缺少Refresh token"
            }))
        ).into_response();
    }
    
    // Note:当前简化实现，生产环境需validaterefresh token有效性
    // 建议：添加token黑名单机制
    let new_token = generate_token("user_from_refresh");
    let new_refresh = generate_refresh_token("user_from_refresh");
    
    info!("Refresh tokensuccess");
    
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "access_token": new_token,
            "refresh_token": new_refresh,
            "token_type": "Bearer",
            "expires_in": 3600
        }))
    ).into_response()
}

/// POST /api/auth/logout
/// 
/// user登出
async fn logout(
    State(_state): State<AuthApiState>,
    headers: HeaderMap,
) -> Response {
    // validate令牌
    let token = extract_token(&headers);
    if token.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "code": "UNAUTHORIZED",
                "message": "未提供Access token"
            }))
        ).into_response();
    }
    
    // Note:当前简化实现，生产环境建议实现token黑名单
    info!("user登出");
    
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Logged out successfully"
        }))
    ).into_response()
}

// ========== 辅助函数 ==========

/// validateEmail格式
fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

/// 哈希Password（使用bcrypt - 企业级安全）
///
/// # Security
/// - 使用bcrypt算法（成本因子12）
/// - 自动生成随机salt
/// - 抗暴力破解
fn hash_password(password: &str) -> String {
    // ✅ 使用环境变量配置bcrypt成本，默认12
    let cost = std::env::var("BCRYPT_COST")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(12);
    bcrypt::hash(password, cost)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to hash password: {}", e);
            // 返回一个永远不匹配的哈希，而不是panic
            "$2b$12$INVALID_HASH_THAT_NEVER_MATCHES".to_string()
        })
}

/// validatePassword（使用bcrypt - 常量时间比较）
///
/// # Security
/// - bcrypt.verify自动使用常量时间比较
/// - 防止时序攻击
fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash)
        .unwrap_or(false)  // validatefailed返回false，不panic
}

/// 生成Access token（使用SHA-256 + 随机盐）
///
/// # Security
/// - 使用SHA-256（安全哈希）
/// - 添加随机盐和时间戳
/// - 不可预测性
fn generate_token(user_id: &str) -> String {
    use sha2::{Sha256, Digest};
    use rand::RngCore;
    
    // 生成随机盐
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    
    // 组合user_id + salt + timestamp
    let timestamp = chrono::Utc::now().timestamp_millis();
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    hasher.update(salt);
    hasher.update(timestamp.to_le_bytes());
    
    format!("token_{:x}", hasher.finalize())
}

fn generate_refresh_token(user_id: &str) -> String {
    use sha2::{Sha256, Digest};
    use rand::RngCore;
    
    // 生成随机盐
    let mut salt = [0u8; 32]; // refresh token 使用更长的盐
    rand::rngs::OsRng.fill_bytes(&mut salt);
    
    // 组合user_id + salt + timestamp
    let timestamp = chrono::Utc::now().timestamp_millis();
    let mut hasher = Sha256::new();
    hasher.update(b"refresh_");
    hasher.update(user_id.as_bytes());
    hasher.update(salt);
    hasher.update(timestamp.to_le_bytes());
    
    format!("refresh_{:x}", hasher.finalize())
}

/// from请求头提取令牌
fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| {
            auth.strip_prefix("Bearer ").map(|s| s.to_string())
        })
}

// verify_token 函数已移除
// 现在使用 state.tokens 映射表进行validate

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("user@example.com"));
        assert!(!is_valid_email("invalid-email"));
    }

    #[test]
    fn test_password_hashing() {
        let password = "test123456";
        let hash = hash_password(password);
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong", &hash));
    }
}

