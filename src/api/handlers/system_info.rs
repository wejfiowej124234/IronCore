use axum::{http::StatusCode, response::Json};
use serde::Serialize;

/// 系统信息响应
#[derive(Debug, Serialize)]
pub struct SystemInfoResponse {
    /// 应用版本
    pub version: String,
    /// 应用名称
    pub name: String,
    /// API版本
    pub api_version: String,
    /// Rust版本
    pub rust_version: String,
    /// 构建时间
    pub build_time: String,
    /// 构建环境
    pub build_profile: String,
}

/// GET /api/system/info
/// 
/// fetch系统信息
pub async fn system_info() -> (StatusCode, Json<SystemInfoResponse>) {
    let info = SystemInfoResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: env!("CARGO_PKG_NAME").to_string(),
        api_version: "v1".to_string(),
        rust_version: rustc_version(),
        build_time: build_time(),
        build_profile: build_profile(),
    };

    (StatusCode::OK, Json(info))
}

/// fetchRust编译器版本
fn rustc_version() -> String {
    option_env!("RUSTC_VERSION")
        .unwrap_or(env!("CARGO_PKG_RUST_VERSION"))
        .to_string()
}

/// fetch构建时间
fn build_time() -> String {
    option_env!("BUILD_TIME")
        .unwrap_or("unknown")
        .to_string()
}

/// fetch构建配置（debug/release）
fn build_profile() -> String {
    if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_info() {
        let (status, json) = system_info().await;
        assert_eq!(status, StatusCode::OK);
        assert!(!json.0.version.is_empty());
        assert_eq!(json.0.name, env!("CARGO_PKG_NAME"));
        assert_eq!(json.0.api_version, "v1");
    }
}

