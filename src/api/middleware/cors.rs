//! CORS 中间件配置
//!
//! 提供跨域资源共享配置

use tower_http::cors::{CorsLayer, AllowOrigin};
use axum::http::{HeaderValue, Method, header};
use std::time::Duration;

/// 创建 CORS 层
///
/// # Arguments
/// * `cors_origin` - 允许的源（可以是逗号分隔的列表）
///
/// # Returns
/// 配置好的 CorsLayer
pub fn create_cors_layer(cors_origin: &str) -> CorsLayer {
    CorsLayer::new()
        .allow_origin({
            if cors_origin.contains(',') {
                // 多个源
                let list = cors_origin
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| HeaderValue::from_str(s)
                        .expect("Invalid CORS origin in list"))
                    .collect::<Vec<HeaderValue>>();
                AllowOrigin::list(list)
            } else {
                // 单个源
                AllowOrigin::exact(
                    HeaderValue::from_str(cors_origin)
                        .expect("Invalid CORS_ALLOW_ORIGIN environment variable")
                )
            }
        })
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::OPTIONS,
            Method::PUT,
            Method::PATCH,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
            header::ACCESS_CONTROL_REQUEST_METHOD,
            header::ACCESS_CONTROL_REQUEST_HEADERS,
        ])
        .expose_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cors_layer_single_origin() {
        let _layer = create_cors_layer("http://localhost:3000");
        // 基本validate：不应该 panic
    }

    #[test]
    fn test_create_cors_layer_multiple_origins() {
        let _layer = create_cors_layer("http://localhost:3000,http://localhost:8080");
        // 基本validate：不应该 panic
    }

    #[test]
    fn test_create_cors_layer_with_spaces() {
        let _layer = create_cors_layer("http://localhost:3000 , http://localhost:8080 ");
        // 应该正确处理空格
    }
}

