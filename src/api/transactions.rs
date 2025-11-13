//! transaction管理 API 路由
//!
//! 提供transaction历史、发送transaction等接口

use axum::{
    extract::{Json, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// transaction记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_id: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub status: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<String>,
}

/// transaction历史query参数
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default)]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub wallet_id: Option<String>,
    pub network: Option<String>,
}

fn default_page_size() -> usize {
    20
}

/// transaction历史响应
#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub items: Vec<Transaction>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

/// 发送transaction请求
#[derive(Debug, Deserialize)]
pub struct SendTransactionRequest {
    pub wallet_id: String,
    pub to: String,
    pub amount: String,
    #[serde(default)]
    pub network: String,
    pub gas_price: Option<String>,
    /// userPassword（用于解密Private keySign transaction）
    pub password: String,
}

/// 发送transaction响应
#[derive(Debug, Serialize)]
pub struct SendTransactionResponse {
    pub tx_id: String,
    pub status: String,
    pub timestamp: String,
}

/// API 状态
#[derive(Clone)]
pub struct TransactionApiState {
    /// transaction历史（使用Arc<Mutex>保证线程安全）
    pub transactions: Arc<Mutex<Vec<Transaction>>>,
    /// API密钥（可选）
    pub api_key: Option<String>,
}

impl TransactionApiState {
    /// Create new API state
    pub fn new() -> Self {
        // 初始化一些示例transaction
        let transactions = vec![
            Transaction {
                tx_id: "0xabc123...".to_string(),
                from: "0x1111...".to_string(),
                to: "0x2222...".to_string(),
                amount: "1.5".to_string(),
                status: "confirmed".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                network: Some("ethereum".to_string()),
                gas_price: Some("20".to_string()),
            },
            Transaction {
                tx_id: "0xdef456...".to_string(),
                from: "0x1111...".to_string(),
                to: "0x3333...".to_string(),
                amount: "0.5".to_string(),
                status: "pending".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                network: Some("ethereum".to_string()),
                gas_price: Some("25".to_string()),
            },
        ];

        Self {
            transactions: Arc::new(Mutex::new(transactions)),
            api_key: None,
        }
    }
}

/// 创建transaction管理路由
pub fn create_transaction_routes(state: TransactionApiState) -> Router {
    Router::new()
        .route("/api/transactions/history", get(get_transaction_history))
        .route("/api/transactions/send", post(send_transaction))
        .with_state(state)
}

/// GET /api/transactions/history
/// 
/// fetchtransaction历史
async fn get_transaction_history(
    State(state): State<TransactionApiState>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    info!("收到transaction历史请求: page={}, page_size={}", query.page, query.page_size);
    
    let transactions = state.transactions.lock().await;
    
    // 过滤和分页
    let mut filtered: Vec<Transaction> = transactions
        .iter()
        .filter(|tx| {
            if let Some(ref wallet_id) = query.wallet_id {
                tx.from.contains(wallet_id) || tx.to.contains(wallet_id)
            } else {
                true
            }
        })
        .filter(|tx| {
            if let Some(ref network) = query.network {
                tx.network.as_ref().map(|n| n == network).unwrap_or(false)
            } else {
                true
            }
        })
        .cloned()
        .collect();
    
    let total = filtered.len();
    
    // 分页
    let start = query.page * query.page_size;
    let end = (start + query.page_size).min(total);
    
    if start < total {
        filtered = filtered[start..end].to_vec();
    } else {
        filtered = vec![];
    }
    
    // 即使没有transaction记录，也返回200状态码 + 空数组
    info!("返回 {} 条transaction记录（共 {} 条）", filtered.len(), total);
    
    if total == 0 {
        info!("当前暂无transaction记录，返回空数组（200状态）");
    }
    
    Json(HistoryResponse {
        items: filtered,
        total,
        page: query.page,
        page_size: query.page_size,
    }).into_response()
}

/// POST /api/transactions/send
/// 
/// 发送transaction（真实的区块链transaction）
async fn send_transaction(
    State(state): State<TransactionApiState>,
    Json(req): Json<SendTransactionRequest>,
) -> Response {
    use axum::http::StatusCode;
    use serde_json::json;
    
    info!("收到发送transaction请求: wallet_id={}, to={}, amount={}, network={}", 
          req.wallet_id, req.to, req.amount, req.network);
    
    // ⚠️ Important安全说明：
    // 当前实现仅用于演示目的。真实的企业级wallet应该：
    // 1. 使用WalletManager来发送真实的区块链transaction
    // 2. 实现速率限制和防重放攻击
    // 3. 实现transaction状态跟踪和确认机制
    // 4. 添加Gas费用估算和限制
    // 5. 实现transactionsign的硬件隔离（如HSM）
    
    // 这里返回error，提示需要集成WalletManager
    let error_response = json!({
        "error": "NotImplemented",
        "message": "真实的以太坊transaction发送需要通过WalletManager集成。当前API仅为演示目的。",
        "details": {
            "required_integration": "需要在此处集成 crate::core::wallet_manager::WalletManager",
            "required_method": "wallet_manager.send_transaction(wallet_name, to, amount, network, password).await",
            "security_note": "Password应通过安全的身份validate系统validate，不应明文存储",
            "implementation_status": "核心transaction发送功能已在 src/core/wallet_manager/transactions.rs 中完整实现"
        },
        "example_integration": {
            "step1": "fetch或注入WalletManager实例",
            "step2": "调用wallet_manager.send_transaction()",
            "step3": "返回真实的Transaction hash"
        }
    });
    
    (StatusCode::NOT_IMPLEMENTED, Json(error_response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_serialization() {
        let tx = Transaction {
            tx_id: "0xabc".to_string(),
            from: "0x111".to_string(),
            to: "0x222".to_string(),
            amount: "1.0".to_string(),
            status: "confirmed".to_string(),
            timestamp: "2025-10-29T12:00:00Z".to_string(),
            network: Some("ethereum".to_string()),
            gas_price: Some("20".to_string()),
        };

        let json = serde_json::to_string(&tx).unwrap();
        assert!(json.contains("\"tx_id\":\"0xabc\""));
    }
}

