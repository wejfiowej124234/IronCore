//! 健康check和指标相关handlers

use serde_json::json;

/// 健康check
pub async fn health_check() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// 监控指标
pub async fn metrics() -> String {
    // Simple metrics stub
    "# HELP wallet_count Number of wallets\n# TYPE wallet_count gauge\nwallet_count 0\n".to_string()
}
