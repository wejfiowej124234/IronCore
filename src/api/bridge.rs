//! 跨链桥 API 路由
//!
//! 提供跨链资产转移接口

use axum::{
    extract::{Json, State},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// 跨链资产转移请求
#[derive(Debug, Deserialize)]
pub struct BridgeAssetsRequest {
    pub from_chain: String,
    pub to_chain: String,
    pub asset: String,
    pub amount: String,
    pub from_address: String,
    pub to_address: String,
}

/// 跨链资产转移响应
#[derive(Debug, Serialize)]
pub struct BridgeAssetsResponse {
    pub bridge_id: String,
    pub status: String,
    pub from_chain: String,
    pub to_chain: String,
    pub amount: String,
    pub estimated_time: String,
    pub timestamp: String,
}

/// 桥接记录
#[derive(Debug, Clone, Serialize)]
pub struct BridgeRecord {
    pub bridge_id: String,
    pub status: String,
    pub from_chain: String,
    pub to_chain: String,
    pub amount: String,
    pub created_at: String,
}

/// API 状态
#[derive(Clone)]
pub struct BridgeApiState {
    /// 桥接记录
    pub records: Arc<Mutex<Vec<BridgeRecord>>>,
    /// API密钥（可选）
    pub api_key: Option<String>,
}

impl BridgeApiState {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(vec![])),
            api_key: None,
        }
    }
}

/// 创建跨链桥路由
pub fn create_bridge_routes(state: BridgeApiState) -> Router {
    Router::new()
        .route("/api/bridge/assets", post(bridge_assets))
        .with_state(state)
}

/// POST /api/bridge/assets
/// 
/// 跨链资产转移
async fn bridge_assets(
    State(state): State<BridgeApiState>,
    Json(req): Json<BridgeAssetsRequest>,
) -> Response {
    info!("收到跨链转移请求: {} -> {}, amount={}", 
          req.from_chain, req.to_chain, req.amount);
    
    // 生成桥接ID（使用SHA-256替代MD5）
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", req.from_chain, req.to_chain, chrono::Utc::now().timestamp_millis()));
    let bridge_id = format!("bridge_{:x}", hasher.finalize());
    
    // 估算时间（根据链的不同给出不同估算）
    let estimated_time = match (req.from_chain.as_str(), req.to_chain.as_str()) {
        ("ethereum", "bsc") | ("bsc", "ethereum") => "3-5 minutes",
        ("bsc", "polygon") | ("polygon", "bsc") => "2-3 minutes",
        _ => "5-15 minutes",
    };
    
    // 创建桥接记录
    let record = BridgeRecord {
        bridge_id: bridge_id.clone(),
        status: "pending".to_string(),
        from_chain: req.from_chain.clone(),
        to_chain: req.to_chain.clone(),
        amount: req.amount.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    
    // 添加到记录
    let mut records = state.records.lock().await;
    records.push(record);
    
    info!("跨链转移已创建: bridge_id={}", bridge_id);
    
    Json(BridgeAssetsResponse {
        bridge_id,
        status: "pending".to_string(),
        from_chain: req.from_chain,
        to_chain: req.to_chain,
        amount: req.amount,
        estimated_time: estimated_time.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_record_serialization() {
        let record = BridgeRecord {
            bridge_id: "bridge_123".to_string(),
            status: "pending".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "bsc".to_string(),
            amount: "1.0".to_string(),
            created_at: "2025-10-29T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"bridge_id\":\"bridge_123\""));
    }
}

