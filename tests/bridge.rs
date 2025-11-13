// tests/bridge.rs - helper that mirrors a bridge handler behavior for tests
use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

use defi_hot_wallet::api::types::{BridgeAssetsRequest, ErrorResponse};
use defi_hot_wallet::core::wallet_manager::WalletManager;

pub async fn bridge_assets(
    State(wallet_manager): State<Arc<WalletManager>>,
    Json(request): Json<BridgeAssetsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // If the wallet does not exist in the state, return a 404 error.
    // This is crucial for testing the 'wallet_not_found' scenario.
    if !wallet_manager
        .list_wallets()
        .await
        .unwrap_or_default()
        .iter()
        .any(|w| w.name == request.from_wallet)
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Wallet not found".to_string(),
                code: "WALLET_NOT_FOUND".to_string(),
            }),
        ));
    }

    info!("Test bridge called: {} -> {}", request.from_chain, request.to_chain);

    match wallet_manager
        .bridge_assets(
            &request.from_wallet,
            &request.from_chain,
            &request.to_chain,
            &request.token,
            &request.amount,
        )
        .await
    {
        Ok(bridge_tx_id) => Ok(Json(json!({ "bridge_tx_id": bridge_tx_id }))),
        Err(e) => {
            warn!("bridge failed: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: e.to_string(), code: "BRIDGE_FAILED".to_string() }),
            ))
        }
    }
}
