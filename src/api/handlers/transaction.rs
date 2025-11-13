//! transaction相关handlers

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::middleware::authenticate;
use crate::api::middleware::extract_user::{extract_user_id_from_token, verify_wallet_ownership};
use crate::api::server::WalletServer;
use crate::api::types::*;
use crate::core::validation::{validate_address, validate_amount};
use crate::api::validators::{validate_wallet_name, validate_transaction_amount, validate_wallet_address};

pub async fn send_transaction(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(query_params): Query<std::collections::HashMap<String, String>>,
    Json(payload): Json<SendTransactionRequest>,
) -> Result<Json<TransactionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // ✅ 使用新的user认证机制
    let user_id = extract_user_id_from_token(&headers, &state).await?;
    
    // ✅ validatewallet属于该user（权限check）
    verify_wallet_ownership(&user_id, &name, &state).await?;

    // validateWallet name（使用共享validate器）
    validate_wallet_name(&name)?;
    
    // ✅ validatetransaction金额
    validate_transaction_amount(&payload.amount)?;
    
    // ✅ validate目标address格式
    validate_wallet_address(&payload.to)?;

    // ✅ 非托管模式：wallet存在性已由verify_wallet_ownershipvalidate（第29行）
    // 不再需要checkwallet_manager，因为非托管wallet不在那里
    
    // Network 参数：优先使用 query，没有则用 body（兼容两种方式）
    let network = query_params.get("network")
        .map(|s| s.as_str())
        .unwrap_or(&payload.network);

    // Validate required parameters after wallet exists
    if payload.to.is_empty() || payload.amount.is_empty() || network.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing required parameters".to_string(),
                code: "TRANSACTION_FAILED".to_string(),
            }),
        ));
    }

    // Validate amount
    if let Err(_e) = validate_amount(&payload.amount) {
        // ✅ 不泄漏validate细节
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid amount format".to_string(),
                code: "TRANSACTION_FAILED".to_string(),
            }),
        ));
    }

    // Validate address format based on network
    if let Err(_e) = validate_address(&payload.to, network) {
        // ✅ 不泄漏validate细节
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid address format".to_string(),
                code: "TRANSACTION_FAILED".to_string(),
            }),
        ));
    }

    // Validate network support（支持前端约定的 canonical 值）
    if !matches!(
        network,
        "ethereum" | "bitcoin" | "polygon" | "bsc" |
        "eth" | "sepolia" | "bsctestnet" | "polygon-testnet"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Unsupported network".to_string(),
                code: "TRANSACTION_FAILED".to_string(),
            }),
        ));
    }

    // ✅ 非托管模式：check是否提供了已Sign transaction
    if let Some(signed_tx) = &payload.signed_tx {
        // 非托管模式：广播已Sign transaction
        tracing::info!("✅ 非托管模式：广播已Sign transaction, wallet={}, network={}", name, network);
        
        let tx_hash = broadcast_signed_transaction(signed_tx, network).await
            .map_err(|e| {
                tracing::error!("广播transactionfailed: {}", e);
                use crate::security::error_sanitizer::sanitize_error_message;
                let safe_msg = sanitize_error_message(&e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: safe_msg,
                        code: "BROADCAST_FAILED".to_string(),
                    }),
                )
            })?;
        
        return Ok(Json(TransactionResponse { 
            tx_id: tx_hash.clone(), 
            tx_hash: Some(tx_hash.clone()), 
            status: "sent".to_string(),
            network: network.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            fee: "0.0".to_string(),
            confirmations: "0".to_string(),
        }));
    }
    
    // 托管模式（兼容旧版）：需要password
    let password = payload.password.as_ref()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "非托管模式需要提供signed_tx，托管模式需要提供password".to_string(),
                    code: "MISSING_PARAMETER".to_string(),
                }),
            )
        })?;

    match state
        .wallet_manager
        .send_transaction(&name, &payload.to, &payload.amount, network, password)
        .await
    {
        Ok(tx_hash) => Ok(Json(TransactionResponse { 
            tx_id: tx_hash.clone(), 
            tx_hash: Some(tx_hash.clone()), 
            status: "sent".to_string(),
            network: network.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            fee: "0.0".to_string(),
            confirmations: "0".to_string(),
        })),
        Err(e) => {
            use crate::security::error_sanitizer::sanitize_error_message;
            let safe_msg = sanitize_error_message(&e.to_string());
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: safe_msg,
                    code: "TRANSACTION_FAILED".to_string(),
                }),
            ))
        },
    }
}

/// 广播已Sign transaction到区块链
async fn broadcast_signed_transaction(signed_tx: &str, network: &str) -> Result<String, String> {
    // ✅ 非托管模式：广播已Sign transaction
    // 实际生产环境应该调用：
    // - Ethereum: eth_sendRawTransaction RPC
    // - Bitcoin: sendrawtransaction RPC
    
    tracing::info!("✅ 广播已Sign transaction: network={}, tx_length={}", network, signed_tx.len());
    
    // TODO: 集成真实的区块链RPC
    // 当前返回模拟Transaction hash
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_millis();
    
    let tx_hash = match network {
        "eth" | "ethereum" | "sepolia" | "polygon" | "bsc" => {
            format!("0x{:064x}", timestamp)
        }
        "btc" | "bitcoin" => {
            format!("{:064x}", timestamp)
        }
        _ => format!("mock_tx_{:x}", timestamp),
    };
    
    Ok(tx_hash)
}

pub async fn get_transaction_history(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<TransactionHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    authenticate(&headers, &state.api_key).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                code: "AUTH_FAILED".to_string(),
            }),
        )
    })?;

    // Validate wallet name
    if name.is_empty() || name.contains(|c: char| !c.is_alphanumeric() && c != '_') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid wallet name".to_string(),
                code: "HISTORY_FAILED".to_string(),
            }),
        ));
    }

    match state.wallet_manager.list_wallets().await {
        Ok(wallets) => {
            if !wallets.iter().any(|w| w.name == name) {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "Wallet not found".to_string(),
                        code: "HISTORY_FAILED".to_string(),
                    }),
                ));
            }
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to check wallet".to_string(),
                    code: "HISTORY_FAILED".to_string(),
                }),
            ))
        }
    }

    match state.wallet_manager.get_transaction_history(&name).await {
        Ok(history) => Ok(Json(TransactionHistoryResponse { transactions: history })),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to get history".to_string(),
                code: "HISTORY_FAILED".to_string(),
            }),
        )),
    }
}

/// GET /api/transactions/history?wallet_name=<name>
pub async fn transactions_history(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<TransactionHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    authenticate(&headers, &state.api_key).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                code: "AUTH_FAILED".to_string(),
            }),
        )
    })?;

    let wallet_name = params.get("wallet_name")
        .ok_or_else(|| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing wallet_name query parameter".to_string(),
                code: "MISSING_PARAMETER".to_string(),
            }),
        ))?;

    if wallet_name.is_empty() || wallet_name.contains(|c: char| !c.is_alphanumeric() && c != '_') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid wallet name".to_string(),
                code: "INVALID_INPUT".to_string(),
            }),
        ));
    }

    match state.wallet_manager.get_transaction_history(wallet_name).await {
        Ok(history) => Ok(Json(TransactionHistoryResponse { transactions: history })),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to get history".to_string(),
                code: "HISTORY_FAILED".to_string(),
            }),
        )),
    }
}

/// POST /api/transactions/send
#[derive(Deserialize)]
pub struct TransactionSendRequest {
    pub wallet_name: String,
    pub to: String,
    pub amount: String,
    pub network: String,
    pub password: String,
}

pub async fn transactions_send(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Json(req): Json<TransactionSendRequest>,
) -> Result<Json<SendTransactionResponse>, (StatusCode, Json<ErrorResponse>)> {
    authenticate(&headers, &state.api_key).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                code: "AUTH_FAILED".to_string(),
            }),
        )
    })?;

    if req.wallet_name.is_empty() || req.to.is_empty() || req.amount.is_empty() || req.network.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing required parameters".to_string(),
                code: "MISSING_PARAMETERS".to_string(),
            }),
        ));
    }

    match state.wallet_manager.send_transaction(
        &req.wallet_name,
        &req.to,
        &req.amount,
        &req.network,
        &req.password,
    ).await {
        Ok(tx_hash) => Ok(Json(SendTransactionResponse {
            tx_hash,
            message: "Transaction sent successfully".to_string(),
        })),
        Err(e) => {
            use crate::security::error_sanitizer::sanitize_error_message;
            let safe_msg = sanitize_error_message(&e.to_string());
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: safe_msg,
                    code: "TRANSACTION_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// GET /api/transactions/:id/status
#[derive(Serialize)]
pub struct TransactionStatusResponse {
    pub tx_id: String,
    pub status: String,
    pub confirmations: u64,
    pub message: String,
}

pub async fn transaction_status(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Path(tx_id): Path<String>,
) -> Result<Json<TransactionStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    authenticate(&headers, &state.api_key).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                code: "AUTH_FAILED".to_string(),
            }),
        )
    })?;

    if tx_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid transaction ID".to_string(),
                code: "INVALID_INPUT".to_string(),
            }),
        ));
    }

    // ✅ 实现基于时间的模拟transaction状态
    // 真实实现需要query区块链RPC，这里提供一个智能模拟
    
    // fromtx_id提取时间戳（如果是我们生成的格式）
    let is_our_tx = tx_id.contains("_mock_") || tx_id.starts_with("eth_") || tx_id.starts_with("btc_");
    
    if is_our_tx {
        // fromtx_id中提取时间戳（hex格式）
        let time_hex = tx_id.split('_').next_back().unwrap_or("0");
        if let Ok(timestamp) = u64::from_str_radix(time_hex, 16) {
            // 计算经过的毫秒数
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::from_secs(0))
                .as_millis() as u64;
            let tx_ms = timestamp;
            let elapsed_ms = now_ms.saturating_sub(tx_ms);
            
            // 模拟确认进度：每15秒增加1个确认
            let confirmations = (elapsed_ms / 15000).min(12);
            let status = if confirmations >= 6 {
                "confirmed"
            } else if confirmations > 0 {
                "pending"
            } else {
                "pending"
            };
            
            return Ok(Json(TransactionStatusResponse {
                tx_id: tx_id.clone(),
                status: status.to_string(),
                confirmations,
                message: if confirmations >= 6 {
                    "Transaction confirmed".to_string()
                } else {
                    format!("Waiting for confirmations ({}/6)", confirmations)
                },
            }));
        }
    }
    
    // 对于外部Transaction hash或未知格式，返回pending
    // Note:真实query需要区块链RPC节点，当前返回pending状态
    Ok(Json(TransactionStatusResponse {
        tx_id: tx_id.clone(),
        status: "pending".to_string(),
        confirmations: 0,
        message: "Transaction statusquery中...（提示：完整实现需要集成区块链RPC）".to_string(),
    }))
}
