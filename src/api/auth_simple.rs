//! Simplified user authentication API (including password recovery)
//! 
//! Features:
//! - Registration/Login
//! - Password recovery/reset
//! - Uses separate users.db database

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{post, get},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::info;

use super::user_db::{UserDatabase, CreateUserRequest as DbCreateUserRequest, LoginRequest as DbLoginRequest};

/// User information (simplified version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: Option<String>,
    pub created_at: String,
    pub last_login: Option<String>,
    pub wallets: Vec<String>,
}

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email_or_wallet_id: String,
    pub password: String,
}

/// Password reset request
#[derive(Debug, Deserialize)]
pub struct RequestPasswordResetRequest {
    pub email: String,
}

/// Reset password request
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

/// Verify reset token request
#[derive(Debug, Deserialize)]
pub struct VerifyResetTokenQuery {
    pub token: String,
}

/// Send email verification request
#[derive(Debug, Deserialize)]
pub struct SendVerificationRequest {
    pub email: String,
}

/// Verify email request
#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

/// Update profile request
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub avatar: Option<String>,
}

/// Update email request
#[derive(Debug, Deserialize)]
pub struct UpdateEmailRequest {
    pub new_email: String,
    pub current_password: String,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
    pub user: User,
    pub token_type: String,
    pub expires_in: u64,
}

/// API state
#[derive(Clone)]
pub struct AuthApiState {
    pub user_db: Arc<UserDatabase>,
    pub session_store: Arc<crate::api::session_store::SessionStore>, // Session storage
    /// Password reset token cache (token -> email)
    pub reset_tokens: Arc<Mutex<HashMap<String, (String, std::time::SystemTime)>>>,
}

impl AuthApiState {
    pub async fn new(
        database_url: &str,
        session_store: Arc<crate::api::session_store::SessionStore>,
    ) -> anyhow::Result<Self> {
        let user_db = UserDatabase::new(database_url).await?;
        user_db.ensure_demo_user().await?;
        
        Ok(Self {
            user_db: Arc::new(user_db),
            session_store,  // Shared SessionStore
            reset_tokens: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

/// Create authentication routes
pub fn create_auth_routes(state: AuthApiState) -> Router {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/request-password-reset", post(request_password_reset))
        .route("/api/auth/reset-password", post(reset_password))
        .route("/api/auth/verify-reset-token", post(verify_reset_token))
        .route("/api/auth/send-verification", post(send_email_verification))
        .route("/api/auth/verify-email", post(verify_email))
        .route("/api/auth/resend-verification", post(resend_verification))
        .route("/api/auth/update-profile", post(update_profile))
        .route("/api/auth/update-email", post(update_email))
        .route("/api/auth/enable-2fa", post(enable_2fa))
        .route("/api/auth/verify-2fa", post(verify_2fa))
        .route("/api/auth/disable-2fa", post(disable_2fa))
        .route("/api/auth/sessions", get(get_sessions))
        .with_state(state)
}

/// POST /api/auth/register
async fn register(
    State(state): State<AuthApiState>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    info!("Received registration request: email={}", req.email);
    tracing::info!("   Password length: {} characters", req.password.len());
    tracing::info!("   Password first 5 characters: {}", &req.password[..req.password.len().min(5)]);
    
    if req.password != req.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Password mismatch",
                "message": "Passwords do not match"
            }))
        ).into_response();
    }
    
    let db_req = DbCreateUserRequest {
        email: req.email.clone(),
        password: req.password,
        username: None,
    };
    
    match state.user_db.create_user(db_req).await {
        Ok(user) => {
            info!("User registration successful: id={}, email={}", user.id, user.email);
            
            let access_token = generate_token(&user.id);
            
            // ✅ 关键修复：注册token到SessionStore（1小时有效期）
            state.session_store.register_token(&access_token, &user.id, 3600).await;
            info!("✅ 注册userToken已注册到会话: user_id={}", user.id);
            
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
            
            // 打印详细error日志
            tracing::error!("❌ 注册failed: {}", error_msg);
            tracing::error!("❌ error详情: {:?}", e);
            
            // ✅ 增强error处理：所有Password和Emailvalidateerror都返回400
            // ⚠️ Note:必须先check"Email already registered"（包含"Email"），再check通用的"email"/"Email"
            let (status, error_type, message) = if error_msg.contains("Email already registered") {
                (StatusCode::CONFLICT, "Email exists", error_msg.clone()) // ✅ 使用user_db返回的完整error消息
            } else if error_msg.contains("UNIQUE constraint failed") {
                (StatusCode::CONFLICT, "Email exists", "Email already registered, please use a different email".to_string())
            } else if error_msg.contains("Email") || error_msg.contains("email") || error_msg.contains("Email") {
                (StatusCode::BAD_REQUEST, "Invalid email", error_msg.clone())
            } else if error_msg.contains("Password") || error_msg.contains("password") || error_msg.contains("Password") {
                (StatusCode::BAD_REQUEST, "Weak password", error_msg.clone())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Registration failed", "Registration failed, please try again later".to_string())
            };
            
            (
                status,
                Json(serde_json::json!({
                    "error": error_type,
                    "message": message,
                    "details": if std::env::var("DEV_MODE").unwrap_or_default() == "1" {
                        Some(error_msg)
                    } else {
                        None
                    }
                }))
            ).into_response()
        }
    }
}

/// POST /api/auth/login
async fn login(
    State(state): State<AuthApiState>,
    Json(req): Json<LoginRequest>,
) -> Response {
    info!("收到Login request: identifier={}", req.email_or_wallet_id);
    tracing::info!("   Password长度: {} 字符", req.password.len());
    tracing::info!("   Password前5字符: {}", &req.password[..req.password.len().min(5)]);
    
    let db_req = DbLoginRequest {
        email: req.email_or_wallet_id.clone(),
        password: req.password,
    };
    
    match state.user_db.verify_login(db_req).await {
        Ok(user) => {
            info!("user登录success: id={}, email={}", user.id, user.email);
            
            let access_token = generate_token(&user.id);
            let wallets = state.user_db.get_user_wallets(&user.id).await.unwrap_or_default();
            
            // ✅ 注册token到SessionStore（1小时有效期）
            state.session_store.register_token(&access_token, &user.id, 3600).await;
            info!("✅ Token已注册到会话: user_id={}", user.id);
            
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
            
            // 打印详细error日志
            tracing::error!("❌ 登录failed: email={}, error={}", req.email_or_wallet_id, error_msg);
            tracing::error!("❌ error详情: {:?}", e);
            
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
                    "message": message,
                    "details": if std::env::var("DEV_MODE").unwrap_or_default() == "1" {
                        Some(error_msg)
                    } else {
                        None
                    }
                }))
            ).into_response()
        }
    }
}

/// POST /api/auth/logout
async fn logout(
    State(state): State<AuthApiState>,
    headers: axum::http::HeaderMap,
) -> Response {
    // ✅ fromheader提取token并撤销
    if let Some(token) = crate::api::middleware::extract_user::extract_token(&headers) {
        if let Ok(user_id) = state.session_store.validate_token(&token).await {
            // Clear会话
            state.session_store.clear_user_session(&user_id).await;
            state.session_store.revoke_token(&token).await.ok();
            info!("✅ user已登出: user_id={}", user_id);
        }
    }
    
    (StatusCode::OK, Json(serde_json::json!({
        "message": "Logged out successfully"
    }))).into_response()
}

/// POST /api/auth/request-password-reset
/// 
/// 请求Password重置（发送重置邮件）
async fn request_password_reset(
    State(state): State<AuthApiState>,
    Json(req): Json<RequestPasswordResetRequest>,
) -> Response {
    info!("收到Password重置请求: email={}", req.email);
    
    // 1. checkuser是否存在（不泄露user是否存在）
    // 2. 生成重置Token
    let reset_token = generate_reset_token();
    let expires_at = std::time::SystemTime::now() + std::time::Duration::from_secs(3600); // 1小时有效
    
    // 3. 保存Token
    state.reset_tokens.lock().await.insert(reset_token.clone(), (req.email.clone(), expires_at));
    
    info!("✅ Password重置Token已生成: {}", reset_token);
    
    // 4. 发送邮件（Mock版本：返回Token给前端）
    // TODO: 生产环境应该通过邮件发送，不返回token
    (StatusCode::OK, Json(serde_json::json!({
        "message": "Password重置邮件已发送",
        "reset_token": reset_token,  // Mock: 实际应该通过邮件发送
        "reset_link": format!("http://localhost:3012/reset-password?token={}", reset_token)
    }))).into_response()
}

/// POST /api/auth/verify-reset-token
/// 
/// validate重置Token是否有效
async fn verify_reset_token(
    State(state): State<AuthApiState>,
    Json(query): Json<VerifyResetTokenQuery>,
) -> Response {
    let tokens = state.reset_tokens.lock().await;
    
    if let Some((email, expires_at)) = tokens.get(&query.token) {
        if *expires_at > std::time::SystemTime::now() {
            return (StatusCode::OK, Json(serde_json::json!({
                "valid": true,
                "email": email
            }))).into_response();
        }
    }
    
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({
        "error": "Invalid or expired token",
        "message": "重置链接无效或已过期"
    }))).into_response()
}

/// POST /api/auth/reset-password
/// 
/// 使用Token重置Password
async fn reset_password(
    State(state): State<AuthApiState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Response {
    info!("收到重置Password请求");
    
    // 1. validateToken
    let mut tokens = state.reset_tokens.lock().await;
    let email = match tokens.get(&req.token) {
        Some((email, expires_at)) if *expires_at > std::time::SystemTime::now() => {
            email.clone()
        }
        _ => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid or expired token",
                "message": "重置链接无效或已过期"
            }))).into_response();
        }
    };
    
    // 2. 更新Password（调用user_db）
    match state.user_db.update_password(&email, &req.new_password).await {
        Ok(_) => {
            // 3. Delete已使用的Token
            tokens.remove(&req.token);
            drop(tokens);
            
            info!("✅ Password重置success: email={}", email);
            
            (StatusCode::OK, Json(serde_json::json!({
                "message": "Password重置success，请使用新Password登录"
            }))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Reset failed",
                "message": format!("Password重置failed: {}", e)
            }))).into_response()
        }
    }
}

/// 生成简单的Access token  
fn generate_token(user_id: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_else(|_| 0);
    format!("token_{}_{}", user_id, timestamp)
}

/// 生成Password重置Token
fn generate_reset_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_else(|_| 0);
    format!("reset_{:x}", timestamp)
}

/// POST /api/auth/send-verification
/// 发送Emailvalidate码
async fn send_email_verification(
    State(_state): State<AuthApiState>,
    Json(req): Json<SendVerificationRequest>,
) -> Response {
    info!("发送Emailvalidate: email={}", req.email);
    
    // Mock实现：返回validate链接
    let verification_token = generate_reset_token();
    
    (StatusCode::OK, Json(serde_json::json!({
        "message": "validate邮件已发送",
        "verification_link": format!("http://localhost:3012/verify-email?token={}", verification_token)
    }))).into_response()
}

/// POST /api/auth/verify-email
/// validateEmail
async fn verify_email(
    State(_state): State<AuthApiState>,
    Json(req): Json<VerifyEmailRequest>,
) -> Response {
    info!("validateEmail: token={}", req.token);
    
    // Mock实现
    (StatusCode::OK, Json(serde_json::json!({
        "message": "Emailvalidatesuccess",
        "email": "user@example.com"
    }))).into_response()
}

/// POST /api/auth/resend-verification
/// 重新发送validate邮件
async fn resend_verification(
    State(_state): State<AuthApiState>,
    Json(req): Json<SendVerificationRequest>,
) -> Response {
    send_email_verification(State(_state), Json(req)).await
}

/// POST /api/auth/update-profile
/// 更新user资料
async fn update_profile(
    State(_state): State<AuthApiState>,
    Json(_req): Json<UpdateProfileRequest>,
) -> Response {
    info!("更新user资料");
    
    // Mock实现
    (StatusCode::OK, Json(serde_json::json!({
        "message": "资料更新success"
    }))).into_response()
}

/// POST /api/auth/update-email
/// 更新Email
async fn update_email(
    State(_state): State<AuthApiState>,
    Json(req): Json<UpdateEmailRequest>,
) -> Response {
    info!("更新Email: new_email={}", req.new_email);
    
    // Mock实现
    (StatusCode::OK, Json(serde_json::json!({
        "message": "Email更新success，请validate新Email"
    }))).into_response()
}

/// POST /api/auth/enable-2fa
/// 启用双因素认证
async fn enable_2fa(
    State(_state): State<AuthApiState>,
) -> Response {
    info!("启用2FA");
    
    // 生成TOTP密钥（Mock）
    let secret = format!("MOCK2FA{}", generate_reset_token());
    let qr_code_url = format!("otpauth://totp/SecureWallet?secret={}&issuer=SecureWallet", secret);
    
    (StatusCode::OK, Json(serde_json::json!({
        "secret": secret,
        "qrCodeUrl": qr_code_url,
        "backupCodes": ["123456", "234567", "345678"]
    }))).into_response()
}

/// POST /api/auth/verify-2fa
/// validate2FA码
#[derive(Debug, Deserialize)]
struct Verify2FARequest {
    code: String,
}

async fn verify_2fa(
    State(_state): State<AuthApiState>,
    Json(req): Json<Verify2FARequest>,
) -> Response {
    info!("validate2FA码: {}", req.code);
    
    // Mock实现：任何6位数字都通过
    if req.code.len() == 6 && req.code.chars().all(|c| c.is_numeric()) {
        (StatusCode::OK, Json(serde_json::json!({
            "message": "2FAvalidatesuccess"
        }))).into_response()
    } else {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Invalid code",
            "message": "validate码无效"
        }))).into_response()
    }
}

/// POST /api/auth/disable-2fa
/// 禁用2FA
async fn disable_2fa(
    State(_state): State<AuthApiState>,
) -> Response {
    info!("禁用2FA");
    
    (StatusCode::OK, Json(serde_json::json!({
        "message": "2FA已禁用"
    }))).into_response()
}

/// GET /api/auth/sessions
async fn get_sessions(
    State(_state): State<AuthApiState>,
) -> Response {
    info!("fetch会话列表");
    
    // Mock返回空会话列表
    Json(serde_json::json!({
        "sessions": []
    })).into_response()
}
