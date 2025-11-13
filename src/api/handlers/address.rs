//! address相关handlers

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use std::sync::Arc;
use tracing::{info, error};

// use crate::api::middleware::authenticate; // ✅ 开发环境已禁用认证
use crate::api::middleware::extract_user::{extract_user_id_from_token, verify_wallet_ownership};
use crate::api::server::WalletServer;
use crate::api::types::*;
use crate::api::validators::{validate_wallet_name, validate_and_normalize_network};

/// fetchwalletaddress
pub async fn get_wallet_address(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,  // ✅ 启用user认证
    Path(name): Path<String>,
    Query(query): Query<AddressQuery>,
) -> Result<Json<AddressResponse>, (StatusCode, Json<ErrorResponse>)> {
    // ✅ 提取当前登录User ID
    let user_id = extract_user_id_from_token(&headers, &state).await?;
    
    // ✅ validatewallet属于该user（权限check）
    verify_wallet_ownership(&user_id, &name, &state).await?;

    // validateWallet name（使用共享validate器）
    validate_wallet_name(&name)?;

    // ✅ validate并规范化network参数（使用共享validate器）- 如果未提供，默认使用eth
    let network = query.network.as_deref().unwrap_or("eth");
    let normalized_network = validate_and_normalize_network(network)?;

    // ✅ 非托管模式：fromuser_wallets表fetchwalletaddress
    let wallets = state.user_db.get_user_wallets_with_address(&user_id)
        .await
        .map_err(|e| {
            error!("fetchuserwalletfailed: user_id={}, error={}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "queryuserwalletfailed".to_string(),
                    code: "DB_ERROR".to_string(),
                }),
            )
        })?;

    // 查找指定wallet
    let wallet_info = wallets.iter()
        .find(|w| w.name == name)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Wallet not found".to_string(),
                    code: "WALLET_NOT_FOUND".to_string(),
                }),
            )
        })?;

    let wallet_address = wallet_info.address.as_ref()
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Wallet address not found".to_string(),
                    code: "WALLET_ADDRESS_MISSING".to_string(),
                }),
            )
        })?;

    // ✅ 非托管模式：直接返回存储的address（不需要解密）
    info!("✅ 非托管addressquery: wallet={}, address={}, network={}", 
          name, wallet_address, normalized_network);
    
    Ok(Json(AddressResponse {
        address: wallet_address.clone(),
        network: normalized_network.to_string(),
    }))
}

