//! Alchemy NFT API 集成

use super::types::*;
use crate::core::errors::WalletError;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// Alchemy NFT API 客户端
pub struct AlchemyNFTClient {
    client: Client,
    api_key: String,
    network: String,
}

/// Alchemy NFT 响应（简化）
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AlchemyNFTResponse {
    #[serde(rename = "ownedNfts")]
    owned_nfts: Vec<AlchemyNFT>,
    #[serde(rename = "totalCount")]
    total_count: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AlchemyNFT {
    contract: ContractInfo,
    #[serde(rename = "tokenId")]
    token_id: String,
    title: String,
    description: Option<String>,
    metadata: Option<NFTMetadata>,
    #[serde(rename = "tokenType")]
    token_type: Option<String>,
    balance: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContractInfo {
    address: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NFTMetadata {
    name: Option<String>,
    description: Option<String>,
    image: Option<String>,
    attributes: Option<Vec<MetadataAttribute>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MetadataAttribute {
    trait_type: String,
    value: serde_json::Value,
}

impl AlchemyNFTClient {
    /// 创建新的 Alchemy 客户端
    pub fn new(api_key: String, network: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            api_key,
            network,
        }
    }

    /// fetchwallet的 NFT 列表
    pub async fn get_nfts_for_owner(
        &self,
        owner: &str,
        page: u32,
        limit: u32,
    ) -> Result<NFTListResponse, WalletError> {
        // 构建 URL
        let url = format!(
            "https://{}-mainnet.g.alchemy.com/nft/v2/{}/getNFTs",
            self.network, self.api_key
        );

        // 发送请求
        let response = self
            .client
            .get(&url)
            .query(&[
                ("owner", owner),
                ("pageSize", &limit.to_string()),
                ("pageKey", &((page - 1) * limit).to_string()),
                ("withMetadata", "true"),
            ])
            .send()
            .await
            .map_err(|e| {
                WalletError::NetworkError(format!("Alchemy API 请求failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(WalletError::NetworkError(format!(
                "Alchemy API error {}: {}",
                status, body
            )));
        }

        let data: AlchemyNFTResponse = response
            .json()
            .await
            .map_err(|e| {
                WalletError::NetworkError(format!("解析 Alchemy 响应failed: {}", e))
            })?;

        // 转换为我们的格式
        self.convert_nfts_response(data, page, limit)
    }

    /// 转换 Alchemy 响应
    fn convert_nfts_response(
        &self,
        data: AlchemyNFTResponse,
        page: u32,
        limit: u32,
    ) -> Result<NFTListResponse, WalletError> {
        let nfts: Vec<NFT> = data
            .owned_nfts
            .into_iter()
            .map(|nft| {
                // fetch图片 URL
                let image_url = nft
                    .metadata
                    .as_ref()
                    .and_then(|m| m.image.clone())
                    .unwrap_or_default();

                // fetch名称
                let name = nft
                    .metadata
                    .as_ref()
                    .and_then(|m| m.name.clone())
                    .unwrap_or_else(|| nft.title.clone());

                // fetch描述
                let description = nft
                    .metadata
                    .as_ref()
                    .and_then(|m| m.description.clone())
                    .or(nft.description);

                // 转换属性
                let attributes = nft
                    .metadata
                    .as_ref()
                    .and_then(|m| m.attributes.as_ref())
                    .map(|attrs| {
                        attrs
                            .iter()
                            .map(|attr| NFTAttribute {
                                trait_type: attr.trait_type.clone(),
                                value: attr.value.to_string(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                NFT {
                    id: format!("{}:{}:{}", self.network, nft.contract.address, nft.token_id),
                    contract_address: nft.contract.address,
                    token_id: nft.token_id,
                    name,
                    description,
                    image_url,
                    collection_name: nft.contract.name.unwrap_or_else(|| "Unknown".to_string()),
                    standard: nft.token_type.unwrap_or_else(|| "ERC721".to_string()),
                    attributes,
                    balance: nft.balance,
                }
            })
            .collect();

        Ok(NFTListResponse {
            nfts,
            total: data.total_count,
            page,
            limit,
        })
    }
}

/// from环境变量fetch Alchemy API Key
pub fn get_alchemy_api_key(network: &str) -> Option<String> {
    let env_var = format!("ALCHEMY_API_KEY_{}", network.to_uppercase());
    std::env::var(&env_var).ok()
        .or_else(|| std::env::var("ALCHEMY_API_KEY").ok())
}

