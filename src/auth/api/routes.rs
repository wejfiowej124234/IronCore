//! API路由定义

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::auth::AuthService;
use super::handlers;

/// 创建认证路由
pub fn create_auth_routes(service: Arc<AuthService>) -> Router {
    Router::new()
        // 注册和登录
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/signup", post(handlers::register))  // 别名，兼容前端
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/logout", post(handlers::logout))
        
        // OAuth
        .route("/api/auth/google", post(handlers::google_auth))
        
        // user管理
        .route("/api/auth/me", get(handlers::get_current_user))
        .route("/api/auth/change-password", post(handlers::change_password))
        .route("/api/auth/password", post(handlers::change_password))  // 别名，兼容前端
        
        // 辅助功能
        .route("/api/auth/check-name", get(handlers::check_name))
        
        .with_state(service)
}

