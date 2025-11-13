//! GameFi 和空投相关数据类型

use serde::{Deserialize, Serialize};

/// GameFi 资产列表请求
#[derive(Debug, Deserialize)]
pub struct GameAssetListRequest {
    pub wallet: String,
    #[serde(default = "default_network")]
    pub network: String,
}

fn default_network() -> String {
    "eth".to_string()
}

/// GameFi 资产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAsset {
    /// 资产ID
    pub id: String,
    /// 游戏名称
    pub game_name: String,
    /// 资产类型（character, item, land等）
    pub asset_type: String,
    /// 资产名称
    pub name: String,
    /// 图片URL
    pub image: String,
    /// 价值（USD）
    pub value_usd: f64,
    /// 详细属性
    pub attributes: serde_json::Value,
}

/// GameFi 资产列表响应
#[derive(Debug, Serialize)]
pub struct GameAssetListResponse {
    pub assets: Vec<GameAsset>,
    pub total: u32,
}

/// 空投信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Airdrop {
    /// 空投ID
    pub id: String,
    /// 项目名称
    pub project_name: String,
    /// 代币符号
    pub token_symbol: String,
    /// 空投数量
    pub amount: String,
    /// 价值（USD）
    pub value_usd: f64,
    /// 是否可领取
    pub claimable: bool,
    /// 领取截止时间（Unix时间戳）
    pub claim_deadline: u64,
    /// 领取链接
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_url: Option<String>,
}

/// 空投列表请求
#[derive(Debug, Deserialize)]
pub struct AirdropListRequest {
    pub wallet: String,
    #[serde(default = "default_network")]
    pub network: String,
}

/// 空投列表响应
#[derive(Debug, Serialize)]
pub struct AirdropListResponse {
    pub airdrops: Vec<Airdrop>,
    pub total: u32,
}

/// 领取空投请求
#[derive(Debug, Deserialize)]
pub struct AirdropClaimRequest {
    pub wallet_name: String,
    pub airdrop_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// 领取空投响应
#[derive(Debug, Serialize)]
pub struct AirdropClaimResponse {
    pub tx_id: String,
    pub status: String,
    pub claimed_amount: String,
}

