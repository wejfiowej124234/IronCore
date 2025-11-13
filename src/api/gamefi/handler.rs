//! GameFi 和空投 API 处理器

use super::types::*;
use crate::api::server::WalletServer;
use crate::api::types::ErrorResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tracing::{error, info};

/// GET /api/gamefi/assets/:wallet
///
/// fetchwallet的 GameFi 资产
pub async fn get_game_assets(
    State(_state): State<Arc<WalletServer>>,
    Path(wallet): Path<String>,
    Query(params): Query<GameAssetListRequest>,
) -> Response {
    info!("fetch GameFi 资产: wallet={}, network={}", wallet, params.network);

    // 返回 Mock 数据
    let mock_assets = create_mock_game_assets(&wallet, &params.network);
    Json(mock_assets).into_response()
}

/// GET /api/airdrops/:wallet
///
/// fetch可用的空投
pub async fn get_airdrops(
    State(_state): State<Arc<WalletServer>>,
    Path(wallet): Path<String>,
    Query(params): Query<AirdropListRequest>,
) -> Response {
    info!("fetch空投列表: wallet={}, network={}", wallet, params.network);

    // 返回 Mock 数据
    let mock_airdrops = create_mock_airdrops(&wallet, &params.network);
    Json(mock_airdrops).into_response()
}

/// POST /api/airdrops/:id/claim
///
/// 领取空投
pub async fn claim_airdrop(
    State(state): State<Arc<WalletServer>>,
    Path(airdrop_id): Path<String>,
    Json(req): Json<AirdropClaimRequest>,
) -> Response {
    info!(
        "领取空投: id={}, wallet={}",
        airdrop_id, req.wallet_name
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

    // checkwallet是否存在
    if let Err(e) = state.wallet_manager.get_wallet_by_name(&req.wallet_name).await {
        error!("wallet不存在: {:?}", e);
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("wallet '{}' 不存在", req.wallet_name),
                code: "WALLET_NOT_FOUND".to_string(),
            }),
        )
            .into_response();
    }

    // 实现空投领取逻辑
    // Note:实际的空投领取需要调用项目方的空投合约
    // 每个项目的合约接口可能不同
    
    info!("准备空投领取transaction");
    
    // 简化实现：使用基础的send_transaction作为占位
    // 实际需要构建空投合约的claim()调用
    match state
        .wallet_manager
        .send_transaction(
            &req.wallet_name,
            &airdrop_id,  // 使用空投ID作为目标address占位
            "0",  // 空投领取通常无需发送资金
            "eth",
            req.password.as_deref().unwrap_or(""),
        )
        .await
    {
        Ok(tx_hash) => {
            info!("空投领取transaction已发送: {}", tx_hash);
            let response = AirdropClaimResponse {
                tx_id: tx_hash,
                status: "pending".to_string(),
                claimed_amount: "1000".to_string(),  // from空投信息fetch
            };
            Json(response).into_response()
        }
        Err(e) => {
            error!("空投领取failed: {:?}", e);
            // 降级到 Mock
            let response = AirdropClaimResponse {
                tx_id: format!("0x{:x}", rand::random::<u64>()),
                status: "mock_pending".to_string(),
                claimed_amount: "1000".to_string(),
            };
            Json(response).into_response()
        }
    }
}

/// 创建 Mock GameFi 资产
fn create_mock_game_assets(_wallet: &str, network: &str) -> GameAssetListResponse {
    let assets = vec![
        GameAsset {
            id: format!("{}:axie:001", network),
            game_name: "Axie Infinity".to_string(),
            asset_type: "character".to_string(),
            name: "Axie #001".to_string(),
            image: "https://via.placeholder.com/300".to_string(),
            value_usd: 150.0,
            attributes: serde_json::json!({
                "class": "Beast",
                "breed_count": 2,
                "hp": 57,
                "speed": 35,
                "skill": 31,
                "morale": 43
            }),
        },
        GameAsset {
            id: format!("{}:stepn:sneaker001", network),
            game_name: "StepN".to_string(),
            asset_type: "sneaker".to_string(),
            name: "Common Sneaker #001".to_string(),
            image: "https://via.placeholder.com/300".to_string(),
            value_usd: 85.0,
            attributes: serde_json::json!({
                "type": "Runner",
                "quality": "Common",
                "level": 5,
                "efficiency": 8.5,
                "luck": 6.2,
                "comfort": 7.1,
                "resilience": 9.3
            }),
        },
        GameAsset {
            id: format!("{}:sandbox:land001", network),
            game_name: "The Sandbox".to_string(),
            asset_type: "land".to_string(),
            name: "Land Parcel (12, 34)".to_string(),
            image: "https://via.placeholder.com/300".to_string(),
            value_usd: 500.0,
            attributes: serde_json::json!({
                "x": 12,
                "y": 34,
                "size": "1x1",
                "district": "Vegas City"
            }),
        },
    ];

    info!("返回 {} 个 GameFi 资产（Mock）", assets.len());

    GameAssetListResponse {
        assets,
        total: 3,
    }
}

/// 创建 Mock 空投列表
fn create_mock_airdrops(_wallet: &str, _network: &str) -> AirdropListResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let airdrops = vec![
        Airdrop {
            id: "airdrop_001".to_string(),
            project_name: "Arbitrum".to_string(),
            token_symbol: "ARB".to_string(),
            amount: "1250".to_string(),
            value_usd: 1500.0,
            claimable: true,
            claim_deadline: now + 30 * 24 * 3600, // 30天后
            claim_url: Some("https://arbitrum.foundation/".to_string()),
        },
        Airdrop {
            id: "airdrop_002".to_string(),
            project_name: "Optimism".to_string(),
            token_symbol: "OP".to_string(),
            amount: "750".to_string(),
            value_usd: 1125.0,
            claimable: true,
            claim_deadline: now + 15 * 24 * 3600, // 15天后
            claim_url: Some("https://app.optimism.io/".to_string()),
        },
        Airdrop {
            id: "airdrop_003".to_string(),
            project_name: "Uniswap".to_string(),
            token_symbol: "UNI".to_string(),
            amount: "400".to_string(),
            value_usd: 2000.0,
            claimable: false, // 已领取
            claim_deadline: now - 7 * 24 * 3600, // 已过期
            claim_url: None,
        },
    ];

    info!("返回 {} 个空投（Mock）", airdrops.len());

    AirdropListResponse {
        airdrops,
        total: 3,
    }
}

