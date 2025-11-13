//! 多签相关handlers

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use std::sync::Arc;

use crate::api::middleware::authenticate;
use crate::api::server::WalletServer;
use crate::api::types::*;
use crate::core::validation::{validate_address, validate_amount};

pub async fn rotate_signing_key(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<RotateSigningKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    authenticate(&headers, &state.api_key).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                code: "AUTH_FAILED".to_string(),
            }),
        )
    })?;

    if name.is_empty() || name.contains(|c: char| !c.is_alphanumeric() && c != '_') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid wallet name".to_string(),
                code: "ROTATION_FAILED".to_string(),
            }),
        ));
    }

    match state.wallet_manager.rotate_signing_key(&name).await {
        Ok((old_v, new_v)) => Ok(Json(RotateSigningKeyResponse {
            wallet: name,
            old_version: old_v,
            new_version: new_v,
        })),
        Err(e) => {
            // Avoid logging raw error details which may contain secrets.
            let reveal = std::env::var("DEV_PRINT_SECRETS").ok().as_deref() == Some("1")
                || std::env::var("TEST_SKIP_DECRYPT").ok().as_deref() == Some("1")
                || std::env::var("ALLOW_BRIDGE_MOCKS").ok().as_deref() == Some("1");
            if reveal {
                tracing::warn!("rotate_signing_key failed: {}", e);
            } else {
                tracing::warn!("rotate_signing_key failed: <redacted>");
            }
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to rotate signing key".to_string(),
                    code: "ROTATION_FAILED".to_string(),
                }),
            ))
        }
    }
}

pub async fn send_multi_sig_transaction(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(payload): Json<MultiSigTransactionRequest>,
) -> Result<Json<TransactionResponse>, (StatusCode, Json<ErrorResponse>)> {
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
                code: "MULTI_SIG_FAILED".to_string(),
            }),
        ));
    }

    // Check signatures first (as per test expectations)
    if payload.signatures.len() < state.config.multi_sig_threshold as usize {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Insufficient signatures".to_string(),
                code: "MULTI_SIG_FAILED".to_string(),
            }),
        ));
    }

    // Validate required parameters
    if payload.to.is_empty() || payload.amount.is_empty() || payload.network.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing required parameters".to_string(),
                code: "MULTI_SIG_FAILED".to_string(),
            }),
        ));
    }

    // Validate address format based on network
    if let Err(_e) = validate_address(&payload.to, &payload.network) {
        // ✅ 不泄漏validate细节
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid address format".to_string(),
                code: "MULTI_SIG_FAILED".to_string(),
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
                code: "MULTI_SIG_FAILED".to_string(),
            }),
        ));
    }

    // Validate network support
    if !matches!(
        payload.network.as_str(),
        "eth" | "sepolia" | "bsc" | "polygon-testnet"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Unsupported network".to_string(),
                code: "MULTI_SIG_FAILED".to_string(),
            }),
        ));
    }

    if payload.signatures.len() < state.config.multi_sig_threshold as usize {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Insufficient signatures".to_string(),
                code: "MULTI_SIG_FAILED".to_string(),
            }),
        ));
    }

    let threshold = payload.signatures.len() as u32;
    match state
        .wallet_manager
        .send_multi_sig_transaction(
            &name,
            &payload.to,
            &payload.amount,
            &payload.signatures,
            threshold,
        )
        .await
    {
        Ok(tx_hash) => Ok(Json(TransactionResponse { 
            tx_id: tx_hash.clone(), 
            tx_hash: Some(tx_hash.clone()), 
            status: "sent".to_string(),
            network: payload.network.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            fee: "0.0".to_string(),
            confirmations: "0".to_string(),
        })),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to send multi-sig transaction".to_string(),
                code: "MULTI_SIG_FAILED".to_string(),
            }),
        )),
    }
}

