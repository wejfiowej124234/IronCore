//! AI异常检测 API 服务器（占位实现）
//!
//! 注：完整的异常检测功能尚未实现
//! 提供最小可编译的健康检查端点

use axum::{Router, routing::get};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("🚀 启动AI异常检测API服务器（占位实现）...");
    info!("⚠️  注：完整功能尚未实现");
    
    // 创建简单的健康检查路由
    let app = Router::new()
        .route("/api/health", get(health_check));
    
    // 绑定地址
    let addr = "127.0.0.1:8888";
    info!("📡 监听地址: http://{}", addr);
    info!("📡 可用端点: GET http://{}/api/health", addr);
    
    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("✅ 服务器已启动！");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// GET /api/health
/// 
/// 健康检查端点
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "anomaly-detection-api-placeholder",
        "version": "0.1.0",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "note": "Full anomaly detection features not yet implemented"
    }))
}
