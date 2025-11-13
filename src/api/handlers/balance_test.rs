//! 测试版本的balancequery处理器

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::api::{
    server::WalletServer,
    types::{BalanceResponse, ErrorResponse},
};

/// 最简化的测试版本
pub async fn get_balance_simple(
    State(_state): State<Arc<WalletServer>>,
    Path(name): Path<String>,
) -> Result<Json<BalanceResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(BalanceResponse {
        balance: format!("1000 (test for {})", name),
        network: "eth".to_string(),
        symbol: "ETH".to_string(),
    }))
}

