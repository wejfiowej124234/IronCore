//! user偏好设置API
//! 
//! 提供user个性化设置的存储和管理功能
//! - 最后选择的wallet
//! - 主题设置
//! - 语言设置
//! - 通知开关等

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;

/// 自定义反序列化函数，正确处理Option<Option<String>>
/// - 字段不存在 -> None
/// - 字段存在且值为null -> Some(None)
/// - 字段存在且值为"value" -> Some(Some("value"))
fn deserialize_double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

/// user偏好设置
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPreferences {
    pub user_id: String,
    pub last_selected_wallet: Option<String>,
    pub theme: String,
    pub language: String,
    #[serde(default)]
    pub notifications_enabled: bool,
    #[serde(default)]
    pub two_fa_enabled: bool,
    pub updated_at: i64,
    pub created_at: i64,
}

/// 更新偏好请求  
/// ✅ 完美解决方案：使用自定义反序列化器
#[derive(Debug, Deserialize)]
pub struct UpdatePreferenceRequest {
    /// last_selected_wallet字段的特殊处理：
    /// - 字段未提供 -> None (不更新)
    /// - "last_selected_wallet": null -> Some(None) (更新为null)
    /// - "last_selected_wallet": "value" -> Some(Some("value")) (更新为value)
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub last_selected_wallet: Option<Option<String>>,
    pub theme: Option<String>,
    pub language: Option<String>,
    pub notifications_enabled: Option<bool>,
}

/// API响应包装
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

/// fetchuser偏好
/// 
/// GET /api/users/:user_id/preferences
pub async fn get_user_preferences(
    Extension(user_db): Extension<Arc<crate::api::user_db::UserDatabase>>,
    Path(user_id): Path<String>,
) -> Result<Json<ApiResponse<UserPreferences>>, (StatusCode, Json<ApiResponse<UserPreferences>>)> {
    let pool = user_db.pool();
    // queryuser偏好
    let prefs = sqlx::query_as::<_, UserPreferences>(
        r#"
        SELECT 
            user_id,
            last_selected_wallet,
            theme,
            language,
            notifications_enabled,
            two_fa_enabled,
            updated_at,
            created_at
        FROM user_preferences 
        WHERE user_id = ?
        "#,
    )
    .bind(&user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("[UserPreferences] queryfailed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error("Internal server error".to_string())))
    })?;

    match prefs {
        Some(p) => {
            eprintln!("[UserPreferences] fetchsuccess: user_id={}", user_id);
            Ok(Json(ApiResponse::success(p)))
        }
        None => {
            // ✅ checkuser是否存在（避免外键约束failed）
            let user_exists = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM users WHERE id = ?"
            )
            .bind(&user_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
            
            if user_exists == 0 {
                eprintln!("[UserPreferences] user不存在: user_id={}", user_id);
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("User not found".to_string()))
                ));
            }
            
            // 如果不存在，创建默认偏好
            eprintln!("[UserPreferences] 不存在，创建默认偏好: user_id={}", user_id);
            
            let now = chrono::Utc::now().timestamp();
            let default_prefs = UserPreferences {
                user_id: user_id.clone(),
                last_selected_wallet: None,
                theme: "light".to_string(),
                language: "en-US".to_string(),
                notifications_enabled: true,
                two_fa_enabled: false,
                updated_at: now,
                created_at: now,
            };

            // 插入默认偏好
            sqlx::query(
                r#"
                INSERT INTO user_preferences 
                (user_id, last_selected_wallet, theme, language, notifications_enabled, two_fa_enabled, updated_at, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&user_id)
            .bind(&default_prefs.last_selected_wallet)
            .bind(&default_prefs.theme)
            .bind(&default_prefs.language)
            .bind(if default_prefs.notifications_enabled { 1 } else { 0 })
            .bind(if default_prefs.two_fa_enabled { 1 } else { 0 })
            .bind(now)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|e| {
                eprintln!("[UserPreferences] 创建默认偏好failed: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error("Failed to create default preferences".to_string())))
            })?;

            Ok(Json(ApiResponse::success(default_prefs)))
        }
    }
}

/// 更新user偏好
/// 
/// PUT /api/users/:user_id/preferences
pub async fn update_user_preferences(
    Extension(user_db): Extension<Arc<crate::api::user_db::UserDatabase>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdatePreferenceRequest>,
) -> Result<Json<ApiResponse<UserPreferences>>, (StatusCode, Json<ApiResponse<UserPreferences>>)> {
    let pool = user_db.pool();
    eprintln!(
        "[UserPreferences] 更新请求: user_id={}, last_selected_wallet={:?}",
        user_id, req.last_selected_wallet
    );

    let now = chrono::Utc::now().timestamp();

    // 先确保记录存在（如果不存在会创建）
    let _ = get_user_preferences(Extension(user_db.clone()), Path(user_id.clone())).await?;

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut has_updates = false;

    // ✅ 处理last_selected_wallet（完美支持null）
    // Option<Option<String>>:
    // - None: 字段未提供，不更新
    // - Some(None): 显式设置为null
    // - Some(Some(value)): 设置为具体值
    if req.last_selected_wallet.is_some() {
        updates.push("last_selected_wallet = ?");
        has_updates = true;
    }
    
    if req.theme.is_some() {
        updates.push("theme = ?");
        has_updates = true;
    }
    if req.language.is_some() {
        updates.push("language = ?");
        has_updates = true;
    }
    if req.notifications_enabled.is_some() {
        updates.push("notifications_enabled = ?");
        has_updates = true;
    }

    if !has_updates {
        // 没有更新，直接返回当前偏好
        return get_user_preferences(Extension(user_db), Path(user_id)).await;
    }

    // 更新时间戳
    updates.push("updated_at = ?");

    let sql = format!(
        "UPDATE user_preferences SET {} WHERE user_id = ?",
        updates.join(", ")
    );

    let mut query = sqlx::query(&sql);

    // 按顺序绑定参数
    // ✅ 完美处理Option<Option<String>>
    if let Some(wallet_option) = &req.last_selected_wallet {
        // wallet_option是Option<String>
        // - Some("value"): 绑定"value"
        // - None: 绑定NULL
        query = query.bind(wallet_option.as_deref());
    }
    if let Some(ref theme) = req.theme {
        query = query.bind(theme);
    }
    if let Some(ref lang) = req.language {
        query = query.bind(lang);
    }
    if let Some(notify) = req.notifications_enabled {
        query = query.bind(if notify { 1 } else { 0 });
    }

    query = query.bind(now).bind(&user_id);

    query.execute(pool).await.map_err(|e| {
        eprintln!("[UserPreferences] 更新failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error("Failed to update preferences".to_string())))
    })?;

    eprintln!("[UserPreferences] 更新success: user_id={}", user_id);

    // 返回更新后的偏好
    get_user_preferences(Extension(user_db), Path(user_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response() {
        let success: ApiResponse<String> = ApiResponse::success("OK".to_string());
        assert!(success.success);
        assert_eq!(success.data, Some("OK".to_string()));
        assert_eq!(success.error, None);

        let error: ApiResponse<String> = ApiResponse::error("Error".to_string());
        assert!(!error.success);
        assert_eq!(error.data, None);
        assert_eq!(error.error, Some("Error".to_string()));
    }
}

