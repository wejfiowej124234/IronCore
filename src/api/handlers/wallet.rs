//! wallet管理相关handlers

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::api::middleware::extract_user::{extract_user_id_from_token, verify_wallet_ownership};
use crate::api::server::WalletServer;
use crate::api::types::*;
use crate::api::validators::{validate_wallet_name, validate_wallet_address};
// ✅ 非托管模式：不再需要validate_password_strength（前端不发送Password）

// ✅ 非托管模式：mnemonic由前端生成，此函数不再使用
// /// 生成BIP39mnemonic（支持12或24个单词）
// fn generate_mnemonic_with_word_count(word_count: u32) -> Result<String, String> {
//     use bip39::{Mnemonic, Language};
//     use rand::RngCore;
//     use rand::rngs::OsRng;
//     
//     // validate单词数量
//     let entropy_bits = match word_count {
//         12 => 128, // 128 bits = 16 bytes
//         24 => 256, // 256 bits = 32 bytes
//         _ => return Err(format!("mnemonic数量无效: {}。必须是12或24个单词", word_count)),
//     };
//     
//     let entropy_bytes = entropy_bits / 8;
//     let mut entropy = vec![0u8; entropy_bytes];
//     OsRng.fill_bytes(&mut entropy);
//     
//     let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
//         .map_err(|e| format!("生成mnemonicfailed: {}", e))?;
//     
//     Ok(mnemonic.to_string())
// }

pub async fn create_wallet(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,  // ✅ 启用user认证
    Json(payload): Json<CreateWalletRequest>,
) -> Result<Json<WalletResponse>, (StatusCode, Json<ErrorResponse>)> {
    // ✅ 提取当前登录User ID
    let user_id = extract_user_id_from_token(&headers, &state).await?;

    // validateWallet name（使用共享validate器）
    validate_wallet_name(&payload.name)?;

    // ✅ 非托管模式：walletaddress必须由前端提供
    let wallet_address = payload.wallet_address.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "wallet_address is required in non-custodial mode".to_string(),
                code: "WALLET_ADDRESS_REQUIRED".to_string(),
            }),
        )
    })?;

    // ✅ 使用统一的addressvalidate器
    validate_wallet_address(wallet_address)?;

    // ✅ 非托管模式：直接保存address绑定，不调用WalletManager
    // 将wallet关联到当前user
    let wallet_type = payload.wallet_type.clone().unwrap_or_else(|| "standard".to_string());
    let link_result = state.user_db.link_wallet(
        &user_id, 
        &payload.name,
        wallet_address,
        Some(&wallet_type)
    ).await;
    
    match link_result {
        Ok(_) => {
            info!("✅ 非托管wallet '{}' 已关联到user {} (address: {})", 
                  payload.name, user_id, wallet_address);

            // 构建响应（非托管模式不返回mnemonic，由前端管理）
            let warning = Some("✅ 非托管wallet：您的mnemonic由您自己保管，请务必安全备份！".to_string());

            Ok(Json(WalletResponse {
                id: payload.name.clone(),
                name: payload.name,
                address: wallet_address.clone(),
                quantum_safe: payload.quantum_safe,
                wallet_type: Some(wallet_type.clone()),  // ✅ 返回wallet类型
                mnemonic: None,  // 非托管模式：不返回mnemonic
                warning,
            }))
        }
        Err(e) => {
            // check是否是wallet已存在error - 返回 409 Conflict
            let error_msg = format!("{}", e);
            if error_msg.contains("already exists") {
                return Err((
                    StatusCode::CONFLICT, // 409
                    Json(ErrorResponse {
                        error: format!("Wallet '{}' already exists", payload.name),
                        code: "WALLET_EXISTS".to_string(),
                    }),
                ));
            }
            
            // 其他error
            let test_mode = std::env::var("DEV_MODE").ok().as_deref() == Some("1");
            if test_mode {
                let reveal = std::env::var("DEV_PRINT_SECRETS").ok().as_deref() == Some("1");
                let msg = if reveal {
                    format!("Failed to create wallet: {}", e)
                } else {
                    "Failed to create wallet".to_string()
                };
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: msg, code: "WALLET_CREATION_FAILED".to_string() }),
                ));
            }
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create wallet".to_string(),
                    code: "WALLET_CREATION_FAILED".to_string(),
                }),
            ))
        }
    }
}

/// 创建多签wallet
#[allow(dead_code)]  // ✅ 非托管模式下暂不使用（多签wallet也由前端生成address）
async fn create_multisig_wallet(
    state: Arc<WalletServer>,
    user_id: String,  // ✅ 添加user_id参数
    payload: CreateWalletRequest,
) -> Result<Json<WalletResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("创建多签wallet: {}", payload.name);

    // validate多签配置
    let config = match &payload.multisig_config {
        Some(cfg) => cfg,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "多签wallet需要提供 multisig_config".to_string(),
                    code: "MISSING_MULTISIG_CONFIG".to_string(),
                }),
            ));
        }
    };

    // validate M-of-N 参数
    if config.m == 0 || config.n == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "M 和 N 必须大于 0".to_string(),
                code: "INVALID_MULTISIG_PARAMS".to_string(),
            }),
        ));
    }

    if config.m > config.n {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("M({}) 不能大于 N({})", config.m, config.n),
                code: "INVALID_MULTISIG_PARAMS".to_string(),
            }),
        ));
    }

    // validatesign者数量
    if config.signers.len() != config.n as usize {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("sign者数量({})必须等于 N({})", config.signers.len(), config.n),
                code: "INVALID_SIGNER_COUNT".to_string(),
            }),
        ));
    }

    // validatesign者address
    for (i, signer) in config.signers.iter().enumerate() {
        if signer.address.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("sign者 #{} 的address不能为空", i + 1),
                    code: "INVALID_SIGNER_ADDRESS".to_string(),
                }),
            ));
        }

        if !signer.address.starts_with("0x") || signer.address.len() != 42 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("sign者 #{} 的address格式无效", i + 1),
                    code: "INVALID_SIGNER_ADDRESS".to_string(),
                }),
            ));
        }
    }

    info!("多签配置validate通过: {}-of-{}, {} 个sign者", config.m, config.n, config.signers.len());

    // ✅ 非托管模式：多签wallet也由前端提供address（智能合约address）
    let wallet_address = payload.wallet_address.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "多签wallet需要提供wallet_address（智能合约address）".to_string(),
                code: "WALLET_ADDRESS_REQUIRED".to_string(),
            }),
        )
    })?;

    // validateaddress格式
    if !wallet_address.starts_with("0x") || wallet_address.len() != 42 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid wallet address format".to_string(),
                code: "INVALID_ADDRESS".to_string(),
            }),
        ));
    }

    // ✅ 非托管模式：直接保存address绑定，不调用WalletManager
    match state.user_db.link_wallet(
        &user_id, 
        &payload.name,
        wallet_address,
        Some("multisig")
    ).await {
        Ok(_) => {
            info!("✅ 多签wallet '{}' 已关联到user {} (address: {})", 
                  payload.name, user_id, wallet_address);
            
            // Note:多签配置已存储在wallet数据中
            // 生产环境建议持久化到专门的配置表
            
            let signers_info = config.signers
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {} ({})", i + 1, s.name.as_deref().unwrap_or("未命名"), &s.address[..10]))
                .collect::<Vec<_>>()
                .join(", ");

            Ok(Json(WalletResponse {
                id: payload.name.clone(),
                name: payload.name.clone(),
                address: wallet_address.clone(),  // ✅ 使用前端提供的多签合约address
                quantum_safe: payload.quantum_safe,
                wallet_type: Some("multisig".to_string()),  // ✅ 多签wallet类型
                mnemonic: None,  // 非托管模式：不返回mnemonic
                warning: Some(format!(
                    "✅ 非托管多签wallet：配置{}-of-{}。sign者：{}",
                    config.m, config.n, signers_info
                )),
            }))
        }
        Err(e) => {
            error!("关联多签walletfailed: {:?}", e);
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "创建多签walletfailed".to_string(),
                    code: "MULTISIG_CREATION_FAILED".to_string(),
                }),
            ))
        }
    }
}

pub async fn list_wallets(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,  // ✅ 启用user认证
) -> Result<Json<Vec<WalletResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // ✅ 提取当前登录User ID
    let user_id = extract_user_id_from_token(&headers, &state).await?;
    
    // ✅ 非托管模式：直接fromuser_wallets表fetchwallet信息（包括address）
    let wallets = state.user_db.get_user_wallets_with_address(&user_id)
        .await
        .map_err(|e| {
            error!("fetchuserwallet列表failed: user_id={}, error={}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "queryuserwalletfailed".to_string(),
                    code: "DB_ERROR".to_string(),
                }),
            )
        })?;

    // 转换为WalletResponse
    let user_wallets: Vec<WalletResponse> = wallets
        .into_iter()
        .map(|w| WalletResponse {
            id: w.name.clone(),
            name: w.name.clone(),
            address: w.address.unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string()),
            quantum_safe: false,  // 非托管模式暂不支持量子安全标记
            wallet_type: Some(w.wallet_type.clone()),  // ✅ 返回wallet类型
            mnemonic: None,  // 非托管模式：永不返回mnemonic
            warning: None,
        })
        .collect();
    
    info!("✅ 返回user {} 的 {} 个非托管wallet", user_id, user_wallets.len());
    Ok(Json(user_wallets))
}

pub async fn delete_wallet(
    State(state): State<Arc<WalletServer>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // ✅ 使用新的user认证机制
    let user_id = extract_user_id_from_token(&headers, &state).await?;
    
    // ✅ validatewallet属于该user（权限check）
    verify_wallet_ownership(&user_id, &name, &state).await?;

    // validateWallet name（使用共享validate器）
    validate_wallet_name(&name)?;

    // ✅ 非托管模式：fromuser_wallets表Delete关联
    match state.user_db.unlink_wallet(&user_id, &name).await {
        Ok(deleted) => {
            if deleted {
                info!("✅ wallet关联已Delete: user={}, wallet={}", user_id, name);
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": "Wallet deleted successfully"
                })))
            } else {
                // 未找到wallet
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "Wallet not found".to_string(),
                        code: "WALLET_NOT_FOUND".to_string(),
                    }),
                ))
            }
        }
        Err(e) => {
            error!("Delete Wallet关联failed: user={}, wallet={}, error={}", user_id, name, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete wallet".to_string(),
                    code: "DELETE_WALLET_FAILED".to_string(),
                }),
            ))
        }
    }
}
