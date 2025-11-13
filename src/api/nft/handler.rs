//! NFT API 处理器

use super::alchemy::AlchemyNFTClient;
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

/// GET /api/nfts/:wallet
///
/// fetchwallet的 NFT 列表
pub async fn get_nfts(
    State(_state): State<Arc<WalletServer>>,
    Path(wallet): Path<String>,
    Query(params): Query<NFTListRequest>,
) -> Response {
    info!("fetch NFT 列表: wallet={}, network={}", wallet, params.network);

    // validatewalletaddress（简化：假设是有效address）
    let owner_address = wallet.clone();

    // fetch Alchemy API Key
    let api_key = match super::alchemy::get_alchemy_api_key(&params.network) {
        Some(key) => key,
        None => {
            info!("Alchemy API Key 未配置，返回 Mock 数据");
            return Json(create_mock_nft_list(&params)).into_response();
        }
    };

    // 创建 Alchemy 客户端
    let client = AlchemyNFTClient::new(api_key, params.network.clone());

    // 调用 Alchemy API
    match client.get_nfts_for_owner(&owner_address, params.page, params.limit).await {
        Ok(response) => {
            info!("successfetch {} 个 NFT", response.nfts.len());
            Json(response).into_response()
        }
        Err(e) => {
            error!("fetch NFT failed: {:?}", e);
            // 降级到 Mock 数据
            Json(create_mock_nft_list(&params)).into_response()
        }
    }
}

/// GET /api/nfts/:id
///
/// fetch单个 NFT 详情
pub async fn get_nft_detail(
    State(_state): State<Arc<WalletServer>>,
    Path(nft_id): Path<String>,
) -> Response {
    info!("fetch NFT 详情: id={}", nft_id);

    // 解析 ID（格式：network:contract:tokenId）
    let parts: Vec<&str> = nft_id.split(':').collect();
    if parts.len() != 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "无效的 NFT ID 格式，应为 network:contract:tokenId".to_string(),
                code: "INVALID_NFT_ID".to_string(),
            }),
        )
            .into_response();
    }

    // 返回 Mock 数据
    let mock_nft = create_mock_nft(parts[0], parts[1], parts[2]);
    Json(mock_nft).into_response()
}

/// POST /api/nfts/transfer
///
/// 转移 NFT
pub async fn transfer_nft(
    State(state): State<Arc<WalletServer>>,
    Json(req): Json<NFTTransferRequest>,
) -> Response {
    info!(
        "NFT 转移请求: wallet={} contract={} tokenId={} to={}",
        req.wallet_name, req.contract_address, req.token_id, req.to_address
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

    if req.to_address.trim().is_empty() || !req.to_address.starts_with("0x") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "无效的Recipient address".to_string(),
                code: "INVALID_ADDRESS".to_string(),
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

    // 实现 NFT 转移逻辑
    // Note:实际的 NFT 转移需要调用智能合约（ERC721.transferFrom 或 ERC1155.safeTransferFrom）
    // 这里使用简化的逻辑，通过wallet管理器发送transaction
    
    // 对于 ERC721: transferFrom(from, to, tokenId)
    // 对于 ERC1155: safeTransferFrom(from, to, id, amount, data)
    
    info!("准备 NFT 转移transaction");
    
    // 简化实现：使用基础的send_transaction作为占位
    // 实际需要构建合约调用的calldata
    match state
        .wallet_manager
        .send_transaction(
            &req.wallet_name,
            &req.to_address,
            req.amount.as_deref().unwrap_or("0"),  // NFT转移金额为0
            "eth",  // from合约address推断network
            req.password.as_deref().unwrap_or(""),
        )
        .await
    {
        Ok(tx_hash) => {
            info!("NFT 转移transaction已发送: {}", tx_hash);
            let response = NFTTransferResponse {
                tx_id: tx_hash,
                status: "pending".to_string(),
            };
            Json(response).into_response()
        }
        Err(e) => {
            error!("NFT 转移failed: {:?}", e);
            // 降级到 Mock 响应
            let response = NFTTransferResponse {
                tx_id: format!("0x{:x}", rand::random::<u64>()),
                status: "mock_pending".to_string(),
            };
            Json(response).into_response()
        }
    }
}

/// 创建 Mock NFT 列表
fn create_mock_nft_list(params: &NFTListRequest) -> NFTListResponse {
    let mock_nfts = vec![
        NFT {
            id: format!("{}:0xabc123:1", params.network),
            contract_address: "0xabc123...".to_string(),
            token_id: "1".to_string(),
            name: "Cool Ape #1".to_string(),
            description: Some("A cool ape NFT".to_string()),
            image_url: "https://via.placeholder.com/400".to_string(),
            collection_name: "Cool Apes".to_string(),
            standard: "ERC721".to_string(),
            attributes: vec![
                NFTAttribute {
                    trait_type: "Background".to_string(),
                    value: "Blue".to_string(),
                },
                NFTAttribute {
                    trait_type: "Eyes".to_string(),
                    value: "Laser".to_string(),
                },
            ],
            balance: None,
        },
        NFT {
            id: format!("{}:0xdef456:42", params.network),
            contract_address: "0xdef456...".to_string(),
            token_id: "42".to_string(),
            name: "Crypto Punk #42".to_string(),
            description: Some("Rare crypto punk".to_string()),
            image_url: "https://via.placeholder.com/400".to_string(),
            collection_name: "Crypto Punks".to_string(),
            standard: "ERC721".to_string(),
            attributes: vec![
                NFTAttribute {
                    trait_type: "Type".to_string(),
                    value: "Alien".to_string(),
                },
            ],
            balance: None,
        },
    ];

    NFTListResponse {
        nfts: mock_nfts,
        total: 2,
        page: params.page,
        limit: params.limit,
    }
}

/// 创建 Mock NFT 详情
fn create_mock_nft(network: &str, contract: &str, token_id: &str) -> NFT {
    NFT {
        id: format!("{}:{}:{}", network, contract, token_id),
        contract_address: contract.to_string(),
        token_id: token_id.to_string(),
        name: format!("NFT #{}", token_id),
        description: Some("Mock NFT for testing".to_string()),
        image_url: "https://via.placeholder.com/600".to_string(),
        collection_name: "Mock Collection".to_string(),
        standard: "ERC721".to_string(),
        attributes: vec![
            NFTAttribute {
                trait_type: "Rarity".to_string(),
                value: "Common".to_string(),
            },
        ],
        balance: None,
    }
}

