//! NFT 相关数据类型

use serde::{Deserialize, Serialize};

/// NFT 列表请求参数
#[derive(Debug, Deserialize)]
pub struct NFTListRequest {
    /// walletaddress或名称
    pub wallet: String,
    /// network
    #[serde(default = "default_network")]
    pub network: String,
    /// 页码
    #[serde(default = "default_page")]
    pub page: u32,
    /// 每页数量
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_network() -> String {
    "eth".to_string()
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    20
}

/// NFT 属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFTAttribute {
    pub trait_type: String,
    pub value: String,
}

/// NFT 数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFT {
    /// NFT 唯一ID（格式：network:contract:tokenId）
    pub id: String,
    /// 合约address
    pub contract_address: String,
    /// Token ID
    pub token_id: String,
    /// NFT 名称
    pub name: String,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 图片 URL
    pub image_url: String,
    /// 集合名称
    pub collection_name: String,
    /// 标准（"ERC721" or "ERC1155"）
    pub standard: String,
    /// 属性列表
    #[serde(default)]
    pub attributes: Vec<NFTAttribute>,
    /// 数量（ERC1155）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<String>,
}

/// NFT 列表响应
#[derive(Debug, Serialize)]
pub struct NFTListResponse {
    pub nfts: Vec<NFT>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
}

/// NFT 转移请求
#[derive(Debug, Deserialize)]
pub struct NFTTransferRequest {
    pub wallet_name: String,
    pub contract_address: String,
    pub token_id: String,
    pub to_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 数量（仅用于 ERC1155）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
}

/// NFT 转移响应
#[derive(Debug, Serialize)]
pub struct NFTTransferResponse {
    pub tx_id: String,
    pub status: String,
}

