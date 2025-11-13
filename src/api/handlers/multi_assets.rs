use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

use crate::api::middleware::extract_user::{extract_user_id_from_token, verify_wallet_ownership};
use crate::api::server::WalletServer;
use crate::api::types::ErrorResponse;

/// 多资产balancequery参数
#[derive(Debug, Deserialize)]
pub struct MultiAssetsQuery {
    /// 资产符号列表，逗号分隔（如：BTC,ETH,USDT）
    pub symbols: Option<String>,
}

/// 多资产balance响应
#[derive(Debug, Serialize)]
pub struct MultiAssetsResponse {
    /// Wallet name
    pub wallet: String,
    /// 资产balance映射 (符号 -> balance信息)
    pub balances: HashMap<String, AssetBalance>,
}

/// 单个资产balance信息
#[derive(Debug, Serialize)]
pub struct AssetBalance {
    /// balance
    pub balance: String,
    /// 资产符号
    pub symbol: String,
    /// network
    pub network: String,
}

/// 资产符号到network的映射
fn symbol_to_network(symbol: &str) -> String {
    match symbol.to_uppercase().as_str() {
        "BTC" => "btc",
        "ETH" => "eth",
        "USDT" | "USDC" | "DAI" => "eth", // ERC-20代币默认在以太坊
        "MATIC" => "polygon",
        "BNB" => "bsc",
        _ => "eth", // 默认以太坊
    }
    .to_string()
}

/// GET /api/wallets/:name/assets
/// 
/// querywallet的多个资产balance
pub async fn get_multi_assets(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,  // ✅ 启用user认证
    Path(wallet_name): Path<String>,
    Query(query): Query<MultiAssetsQuery>,
) -> Result<Json<MultiAssetsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // ✅ 提取当前登录User ID
    let user_id = extract_user_id_from_token(&headers, &state).await?;
    
    // ✅ validatewallet属于该user（权限check）
    verify_wallet_ownership(&user_id, &wallet_name, &state).await?;

    // validateWallet name
    if wallet_name.is_empty() || wallet_name.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid wallet name".to_string(),
                code: "INVALID_WALLET_NAME".to_string(),
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
        .find(|w| w.name == wallet_name)
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

    // 解析资产符号列表
    let symbols = if let Some(symbols_str) = query.symbols {
        symbols_str
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    } else {
        // 默认query常见资产
        vec![
            "BTC".to_string(),
            "ETH".to_string(),
            "USDT".to_string(),
            "USDC".to_string(),
        ]
    };

    // query每个资产的balance
    let mut balances = HashMap::new();

    // ✅ 非托管模式：使用address直接query区块链balance（无需Password）
    for symbol in symbols {
        let network = symbol_to_network(&symbol);

        // query区块链balance（模拟）
        match query_blockchain_balance_for_asset(wallet_address, &network, &symbol).await {
            Ok(balance) => {
                balances.insert(
                    symbol.clone(),
                    AssetBalance {
                        balance: balance.to_string(),
                        symbol: symbol.clone(),
                        network: network.clone(),
                    },
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to get balance for {} on {}: {}",
                    symbol,
                    network,
                    e
                );
                // queryfailed时返回0balance，而不是完全failed
                balances.insert(
                    symbol.clone(),
                    AssetBalance {
                        balance: "0".to_string(),
                        symbol: symbol.clone(),
                        network: network.clone(),
                    },
                );
            }
        }
    }

    Ok(Json(MultiAssetsResponse {
        wallet: wallet_name,
        balances,
    }))
}

/// query区块链资产balance（使用address）
async fn query_blockchain_balance_for_asset(
    address: &str,
    network: &str,
    symbol: &str,
) -> Result<String, String> {
    // ✅ 非托管模式：直接用addressquery区块链
    // 实际生产环境应该调用：
    // - Ethereum: eth_getBalance RPC（原生）或 ERC-20合约query（代币）
    // - Bitcoin: 区块链浏览器API
    
    tracing::info!("✅ 非托管多资产query: address={}, network={}, symbol={}", address, network, symbol);
    
    // TODO: 集成真实的区块链RPC
    // 🔧 开发环境：check环境变量决定是否返回测试数据
    let use_test_data = std::env::var("USE_TEST_BALANCE")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true";
    
    if use_test_data {
        // 测试数据（仅用于演示）
        tracing::warn!("⚠️  返回测试balance数据（USE_TEST_BALANCE=true）");
        match (network, symbol) {
            ("eth", "ETH") => Ok("1.5".to_string()),
            ("eth", "USDT") => Ok("100.0".to_string()),
            ("eth", "USDC") => Ok("50.0".to_string()),
            ("eth", "DAI") => Ok("25.0".to_string()),
            ("btc", "BTC") => Ok("0.05".to_string()),
            ("bsc", "BNB") => Ok("0.8".to_string()),
            ("polygon", "MATIC") => Ok("10.2".to_string()),
            _ => Ok("0.0".to_string()),
        }
    } else {
        // 🎯 生产环境：返回真实balance0（需集成区块链RPC）
        tracing::info!("✅ 返回真实balance（当前为0，待集成RPC）");
        Ok("0.0".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_to_network() {
        assert_eq!(symbol_to_network("BTC"), "btc");
        assert_eq!(symbol_to_network("ETH"), "eth");
        assert_eq!(symbol_to_network("USDT"), "eth");
        assert_eq!(symbol_to_network("MATIC"), "polygon");
        assert_eq!(symbol_to_network("BNB"), "bsc");
        assert_eq!(symbol_to_network("UNKNOWN"), "eth"); // 默认
    }
}

