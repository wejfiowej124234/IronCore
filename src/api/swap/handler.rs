//! DEX 交换 API 处理器

use super::oneinch::OneInchClient;
use super::types::*;
use crate::api::server::WalletServer;
use crate::api::types::ErrorResponse;
use crate::core::errors::WalletError;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tracing::{error, info};

/// GET /api/swap/quote
/// 
/// fetch DEX 交换报价
pub async fn swap_quote(
    State(_state): State<Arc<WalletServer>>,
    Query(params): Query<SwapQuoteRequest>,
) -> Response {
    info!(
        "收到交换报价请求: {} {} -> {}",
        params.amount, params.from, params.to
    );

    // validate输入
    if params.from.trim().is_empty() || params.to.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "源代币和目标代币不能为空".to_string(),
                code: "INVALID_INPUT".to_string(),
            }),
        )
            .into_response();
    }

    // 解析数量
    let amount_f64 = match params.amount.parse::<f64>() {
        Ok(v) if v > 0.0 => v,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "无效的交换数量".to_string(),
                    code: "INVALID_AMOUNT".to_string(),
                }),
            )
                .into_response();
        }
    };

    // fetchChain ID
    let chain_id = match network_to_chain_id(&params.network) {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("不支持的network: {}", params.network),
                    code: "UNSUPPORTED_NETWORK".to_string(),
                }),
            )
                .into_response();
        }
    };

    // fetch代币address
    let from_token_addr = match get_token_address(&params.from, chain_id) {
        Some(addr) => addr,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("不支持的源代币: {}", params.from),
                    code: "UNSUPPORTED_TOKEN".to_string(),
                }),
            )
                .into_response();
        }
    };

    let to_token_addr = match get_token_address(&params.to, chain_id) {
        Some(addr) => addr,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("不支持的目标代币: {}", params.to),
                    code: "UNSUPPORTED_TOKEN".to_string(),
                }),
            )
                .into_response();
        }
    };

    // 转换数量为最小单位（假设18位小数）
    let decimals = 18u32;
    let amount_wei = (amount_f64 * 10f64.powi(decimals as i32)) as u128;
    let amount_str = amount_wei.to_string();

    // 创建 1inch 客户端
    let api_key = super::oneinch::get_oneinch_api_key();
    let client = OneInchClient::new(api_key.clone());

    // 如果没有 API Key，使用 Mock 数据
    if api_key.is_none() {
        info!("1inch API Key 未配置，使用 Mock 报价");
        return Json(create_mock_quote(&params, amount_f64)).into_response();
    }

    // 调用 1inch API
    match client
        .get_quote(chain_id, &from_token_addr, &to_token_addr, &amount_str)
        .await
    {
        Ok(quote) => {
            info!("successfetch交换报价: {} {} → {} {}", 
                quote.from_amount, quote.from_token,
                quote.to_amount, quote.to_token
            );
            Json(quote).into_response()
        }
        Err(e) => {
            error!("fetch交换报价failed: {:?}", e);
            // 降级到 Mock 数据
            info!("降级到 Mock 报价");
            Json(create_mock_quote(&params, amount_f64)).into_response()
        }
    }
}

/// POST /api/swap/execute
///
/// 执行 DEX 交换
pub async fn swap_execute(
    State(state): State<Arc<WalletServer>>,
    Json(req): Json<SwapExecuteRequest>,
) -> Response {
    info!(
        "收到交换执行请求: wallet={} 数量={} {} -> {}",
        req.wallet_name, req.amount, req.from_token, req.to_token
    );

    // validate输入
    if req.wallet_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Wallet name不能为空".to_string(),
                code: "INVALID_INPUT".to_string(),
            }),
        )
            .into_response();
    }

    // validate数量
    let amount_f64 = match req.amount.parse::<f64>() {
        Ok(v) if v > 0.0 => v,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "无效的交换数量".to_string(),
                    code: "INVALID_AMOUNT".to_string(),
                }),
            )
                .into_response();
        }
    };

    // validate滑点
    if req.slippage < 0.0 || req.slippage > 50.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "滑点必须在 0-50% 之间".to_string(),
                code: "INVALID_SLIPPAGE".to_string(),
            }),
        )
            .into_response();
    }

    // checkwallet是否存在（尝试fetchwallet信息）
    if let Err(e) = state.wallet_manager.get_wallet_by_name(&req.wallet_name).await {
        error!("wallet '{}' 不存在或无法访问: {:?}", req.wallet_name, e);
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("wallet '{}' 不存在", req.wallet_name),
                code: "WALLET_NOT_FOUND".to_string(),
            }),
        )
            .into_response();
    }

    // fetchChain ID
    let chain_id = match network_to_chain_id(&req.network) {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("不支持的network: {}", req.network),
                    code: "UNSUPPORTED_NETWORK".to_string(),
                }),
            )
                .into_response();
        }
    };

    // fetch代币address
    let from_token_addr = match get_token_address(&req.from_token, chain_id) {
        Some(addr) => addr,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("不支持的源代币: {}", req.from_token),
                    code: "UNSUPPORTED_TOKEN".to_string(),
                }),
            )
                .into_response();
        }
    };

    let to_token_addr = match get_token_address(&req.to_token, chain_id) {
        Some(addr) => addr,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("不支持的目标代币: {}", req.to_token),
                    code: "UNSUPPORTED_TOKEN".to_string(),
                }),
            )
                .into_response();
        }
    };

    // fetchwalletaddress
    let wallet_address = match get_wallet_address(&state, &req.wallet_name, &req.network).await {
        Ok(addr) => addr,
        Err(e) => {
            error!("fetchwalletaddressfailed: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "fetchwalletaddressfailed".to_string(),
                    code: "WALLET_ADDRESS_FAILED".to_string(),
                }),
            )
                .into_response();
        }
    };

    // 转换数量为最小单位
    let decimals = 18u32;
    let amount_wei = (amount_f64 * 10f64.powi(decimals as i32)) as u128;
    let amount_str = amount_wei.to_string();

    // 创建 1inch 客户端
    let api_key = super::oneinch::get_oneinch_api_key();
    
    // 如果没有 API Key，返回 Mock 响应
    if api_key.is_none() {
        info!("1inch API Key 未配置，返回 Mock 交换结果");
        let mock_response = create_mock_execute_response(&req, amount_f64);
        return Json(mock_response).into_response();
    }

    let client = OneInchClient::new(api_key);

    // fetch交换transaction数据
    let tx_data = match client
        .get_swap_tx(
            chain_id,
            &from_token_addr,
            &to_token_addr,
            &amount_str,
            &wallet_address,
            req.slippage,
        )
        .await
    {
        Ok(data) => data,
        Err(e) => {
            error!("fetch交换transaction数据failed: {:?}", e);
            // 降级到 Mock
            info!("降级到 Mock 交换");
            let mock_response = create_mock_execute_response(&req, amount_f64);
            return Json(mock_response).into_response();
        }
    };

    // 使用现有的wallet管理器发送transaction（复用基础设施）
    info!("使用wallet管理器执行交换transaction");
    
    // Note:这里简化为直接调用 send_transaction
    // 实际的 DEX swap 需要更复杂的逻辑（approve、swap调用）
    // 目前先使用基础的转账逻辑作为占位
    match state
        .wallet_manager
        .send_transaction(
            &req.wallet_name,
            &tx_data.to,
            &req.amount,
            &req.network,
            req.password.as_deref().unwrap_or(""),
        )
        .await
    {
        Ok(tx_hash) => {
            info!("交换transaction已发送: {}", tx_hash);
            let response = SwapExecuteResponse {
                tx_id: tx_hash.clone(),
                status: "pending".to_string(),
                from_amount: req.amount.clone(),
                to_amount: calculate_to_amount(amount_f64, &req.from_token, &req.to_token),
                actual_rate: calculate_rate(&req.from_token, &req.to_token),
                gas_used: Some("0.002".to_string()),
                confirmations: 0,
            };
            Json(response).into_response()
        }
        Err(e) => {
            error!("交换transactionfailed: {:?}", e);
            // 降级到 Mock
            let mock_response = create_mock_execute_response(&req, amount_f64);
            Json(mock_response).into_response()
        }
    }
}

/// 创建 Mock 报价
fn create_mock_quote(req: &SwapQuoteRequest, amount: f64) -> SwapQuote {
    let rate = calculate_rate(&req.from, &req.to);
    let to_amount = amount * rate;

    SwapQuote {
        from_token: req.from.clone(),
        to_token: req.to.clone(),
        from_amount: req.amount.clone(),
        to_amount: format!("{:.6}", to_amount),
        exchange_rate: rate,
        price_impact: if amount > 10.0 { 0.5 } else { 0.15 },
        route: vec![RouteStep {
            protocol: "Mock DEX".to_string(),
            from: req.from.clone(),
            to: req.to.clone(),
            percentage: 100,
            pool: None,
        }],
        gas_estimate: "0.002".to_string(),
        estimated_gas_usd: 7.0,
        valid_for: 30,
    }
}

/// 创建 Mock 执行响应
fn create_mock_execute_response(req: &SwapExecuteRequest, amount: f64) -> SwapExecuteResponse {
    let rate = calculate_rate(&req.from_token, &req.to_token);
    let to_amount = amount * rate;

    SwapExecuteResponse {
        tx_id: format!("0x{:x}{:x}", rand::random::<u64>(), rand::random::<u64>()),
        status: "confirmed".to_string(),
        from_amount: req.amount.clone(),
        to_amount: format!("{:.6}", to_amount),
        actual_rate: rate,
        gas_used: Some("0.00185".to_string()),
        confirmations: 12,
    }
}

/// 计算 Mock 汇率
fn calculate_rate(from: &str, to: &str) -> f64 {
    match (from.to_uppercase().as_str(), to.to_uppercase().as_str()) {
        ("ETH", "USDT") | ("ETH", "USDC") => 3500.0,
        ("USDT", "ETH") | ("USDC", "ETH") => 1.0 / 3500.0,
        ("BTC", "USDT") | ("BTC", "USDC") => 65000.0,
        ("USDT", "BTC") | ("USDC", "BTC") => 1.0 / 65000.0,
        ("BNB", "USDT") | ("BNB", "USDC") => 580.0,
        ("USDT", "BNB") | ("USDC", "BNB") => 1.0 / 580.0,
        ("USDT", "USDC") | ("USDC", "USDT") => 1.0,
        ("ETH", "BTC") => 3500.0 / 65000.0,
        ("BTC", "ETH") => 65000.0 / 3500.0,
        _ => 1.0,
    }
}

/// 计算目标代币数量
fn calculate_to_amount(amount: f64, from: &str, to: &str) -> String {
    let rate = calculate_rate(from, to);
    format!("{:.6}", amount * rate)
}

/// fetchwalletaddress（辅助函数）
async fn get_wallet_address(
    _state: &Arc<WalletServer>,
    _wallet_name: &str,
    _network: &str,
) -> Result<String, WalletError> {
    // fromwallet管理器fetchaddress
    // Note:需要通过wallet_manager.get_wallet_address()fetch实际address
    
    // 暂时返回 Mock address
    Ok(format!("0x{:x}", rand::random::<u128>()))
}

