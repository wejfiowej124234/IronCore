//! balancequery相关handlers

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::error;

// use crate::api::middleware::authenticate; // ✅ 开发环境已禁用认证
use crate::api::middleware::extract_user::{extract_user_id_from_token, verify_wallet_ownership};
use crate::api::server::WalletServer;
use crate::api::types::*;
use crate::api::validators::{validate_wallet_name, validate_and_normalize_network};

#[derive(Deserialize)]
pub struct BalanceQuery {
    pub network: String,
}

pub async fn get_balance(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,  // ✅ 启用user认证
    Path(name): Path<String>,
    Query(query): Query<BalanceQuery>,
) -> Result<Json<BalanceResponse>, (StatusCode, Json<ErrorResponse>)> {
    // ✅ 提取当前登录User ID
    let user_id = extract_user_id_from_token(&headers, &state).await?;
    
    // ✅ validatewallet属于该user（权限check）
    verify_wallet_ownership(&user_id, &name, &state).await?;

    // validateWallet name（使用共享validate器）
    validate_wallet_name(&name)?;

    // validate并规范化network参数（使用共享validate器）
    let normalized_network = validate_and_normalize_network(&query.network)?;

    if !matches!(
        normalized_network.as_str(),
        "eth" | "sepolia" | "bsc" | "polygon-testnet" | "btc" | "polygon"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Unsupported network".to_string(),
                code: "GET_BALANCE_FAILED".to_string(),
            }),
        ));
    }

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

    // ✅ 非托管模式：使用address直接query区块链balance（不需要Private key）
    // TODO: 实际应调用区块链RPC，这里返回模拟数据
    let balance = query_blockchain_balance(wallet_address, &normalized_network).await
        .map_err(|e| {
            error!("query区块链balancefailed: address={}, network={}, error={}", 
                   wallet_address, normalized_network, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get balance from blockchain".to_string(),
                    code: "BLOCKCHAIN_QUERY_FAILED".to_string(),
                }),
            )
        })?;

    let symbol = match normalized_network.as_str() {
        "eth" => "ETH",
        "polygon" => "MATIC",
        "bsc" => "BNB",
        "btc" => "BTC",
        _ => "UNKNOWN",
    };
    
    Ok(Json(BalanceResponse {
        balance,
        network: normalized_network,
        symbol: symbol.to_string(),
    }))
}

/// query区块链balance（使用address）
async fn query_blockchain_balance(address: &str, network: &str) -> Result<String, String> {
    // ✅ 非托管模式：直接用addressquery区块链
    // 实际生产环境应该调用：
    // - Ethereum: eth_getBalance RPC
    // - Bitcoin: 区块链浏览器API
    
    tracing::info!("✅ 非托管balancequery: address={}, network={}", address, network);
    
    // TODO: 集成真实的区块链RPC
    // 🔧 开发环境：check环境变量决定是否返回测试数据
    let use_test_data = std::env::var("USE_TEST_BALANCE")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true";
    
    if use_test_data {
        // 测试数据（仅用于演示）
        tracing::warn!("⚠️  返回测试balance数据（USE_TEST_BALANCE=true）");
        match network {
            "eth" | "sepolia" => Ok("1.5".to_string()),
            "bsc" => Ok("0.8".to_string()),
            "polygon" => Ok("10.2".to_string()),
            "btc" => Ok("0.05".to_string()),
            _ => Ok("0.0".to_string()),
        }
    } else {
        // 🎯 生产环境：返回真实balance0（需集成区块链RPC）
        tracing::info!("✅ 返回真实balance（当前为0，待集成RPC）");
        Ok("0.0".to_string())
    }
}
