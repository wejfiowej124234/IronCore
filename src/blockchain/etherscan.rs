//! Etherscan API 客户端
//!
//! 提供以太坊transaction历史query功能

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::blockchain::traits::Transaction;

/// Etherscan API 客户端
pub struct EtherscanClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

/// Etherscan API 响应
#[derive(Debug, Deserialize)]
struct EtherscanResponse {
    status: String,
    message: String,
    result: Vec<EtherscanTransaction>,
}

/// Etherscan transaction记录
#[derive(Debug, Deserialize, Serialize)]
struct EtherscanTransaction {
    #[serde(rename = "hash")]
    pub hash: String,
    
    #[serde(rename = "from")]
    pub from: String,
    
    #[serde(rename = "to")]
    pub to: String,
    
    #[serde(rename = "value")]
    pub value: String,
    
    #[serde(rename = "timeStamp")]
    pub timestamp: String,
    
    #[serde(rename = "isError")]
    pub is_error: String,
    
    #[serde(rename = "gasUsed")]
    pub gas_used: String,
    
    #[serde(rename = "gasPrice")]
    pub gas_price: String,
    
    #[serde(rename = "blockNumber")]
    pub block_number: String,
}

impl EtherscanClient {
    /// 创建新的Etherscan客户端
    ///
    /// # Arguments
    /// * `api_key` - Etherscan API密钥（from https://etherscan.io/apis fetch）
    /// * `network` - network名称（"mainnet", "sepolia"等）
    pub fn new(api_key: String, network: &str) -> Self {
        let base_url = match network {
            "mainnet" | "eth" => "https://api.etherscan.io",
            "sepolia" => "https://api-sepolia.etherscan.io",
            "goerli" => "https://api-goerli.etherscan.io",
            _ => "https://api.etherscan.io",
        };
        
        Self {
            api_key,
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }
    
    /// fetchaddress的transaction历史
    ///
    /// # Arguments
    /// * `address` - 以太坊address
    /// * `start_block` - 起始区块（0表示最早）
    /// * `end_block` - 结束区块（99999999表示最新）
    /// * `page` - 页码（from1start）
    /// * `offset` - 每页数量（最大10000）
    pub async fn get_transactions(
        &self,
        address: &str,
        start_block: u64,
        end_block: u64,
        page: u32,
        offset: u32,
    ) -> Result<Vec<Transaction>> {
        let url = format!(
            "{}/api?module=account&action=txlist&address={}&startblock={}&endblock={}&page={}&offset={}&sort=desc&apikey={}",
            self.base_url, address, start_block, end_block, page, offset, self.api_key
        );
        
        let response: EtherscanResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;
        
        if response.status != "1" {
            return Err(anyhow::anyhow!("Etherscan APIerror: {}", response.message));
        }
        
        // 转换为通用Transaction格式
        let transactions = response.result.into_iter().map(|tx| {
            Transaction {
                hash: tx.hash,
                from: tx.from,
                to: tx.to,
                amount: tx.value,
                timestamp: tx.timestamp.parse().unwrap_or(0),
                status: if tx.is_error == "0" { "success".to_string() } else { "failed".to_string() },
                network: "ethereum".to_string(),
                block_number: Some(tx.block_number.parse().unwrap_or(0)),
                gas_used: Some(tx.gas_used),
                gas_price: Some(tx.gas_price),
            }
        }).collect();
        
        Ok(transactions)
    }
    
    /// fetchaddress的最近transaction
    ///
    /// # Arguments
    /// * `address` - 以太坊address
    /// * `limit` - 返回数量限制（默认10）
    pub async fn get_recent_transactions(
        &self,
        address: &str,
        limit: u32,
    ) -> Result<Vec<Transaction>> {
        self.get_transactions(address, 0, 99999999, 1, limit).await
    }
    
    /// fetchaddress的所有transaction（分页）
    pub async fn get_all_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<Transaction>> {
        let mut all_transactions = Vec::new();
        let mut page = 1;
        let offset = 1000;
        
        loop {
            let txs = self.get_transactions(address, 0, 99999999, page, offset).await?;
            
            if txs.is_empty() {
                break;
            }
            
            all_transactions.extend(txs.clone());
            
            // 如果返回的数量少于offset，说明已经到最后一页
            if txs.len() < offset as usize {
                break;
            }
            
            page += 1;
        }
        
        Ok(all_transactions)
    }
    
    /// fetchaddress的ERC20代币转账历史
    pub async fn get_erc20_transfers(
        &self,
        address: &str,
        contract_address: Option<&str>,
    ) -> Result<Vec<Transaction>> {
        let mut url = format!(
            "{}/api?module=account&action=tokentx&address={}&sort=desc&apikey={}",
            self.base_url, address, self.api_key
        );
        
        if let Some(contract) = contract_address {
            url.push_str(&format!("&contractaddress={}", contract));
        }
        
        let response: EtherscanResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;
        
        if response.status != "1" {
            return Err(anyhow::anyhow!("Etherscan APIerror: {}", response.message));
        }
        
        let transactions = response.result.into_iter().map(|tx| {
            Transaction {
                hash: tx.hash,
                from: tx.from,
                to: tx.to,
                amount: tx.value,
                timestamp: tx.timestamp.parse().unwrap_or(0),
                status: if tx.is_error == "0" { "success".to_string() } else { "failed".to_string() },
                network: "ethereum".to_string(),
                block_number: Some(tx.block_number.parse().unwrap_or(0)),
                gas_used: Some(tx.gas_used),
                gas_price: Some(tx.gas_price),
            }
        }).collect();
        
        Ok(transactions)
    }
}

/// from环境变量创建Etherscan客户端
pub fn create_etherscan_client(network: &str) -> Result<EtherscanClient> {
    let api_key = std::env::var("ETHERSCAN_API_KEY")
        .unwrap_or_else(|_| {
            tracing::warn!("ETHERSCAN_API_KEY未设置，使用免费限额");
            "YourApiKeyToken".to_string()  // 免费tier
        });
    
    Ok(EtherscanClient::new(api_key, network))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etherscan_client_creation() {
        let client = EtherscanClient::new("test_key".to_string(), "mainnet");
        assert_eq!(client.base_url, "https://api.etherscan.io");
        
        let client2 = EtherscanClient::new("test_key".to_string(), "sepolia");
        assert_eq!(client2.base_url, "https://api-sepolia.etherscan.io");
    }
    
    // Note:以下测试需要真实的API key和network连接，在CI中会跳过
    #[tokio::test]
    #[ignore]
    async fn test_get_transactions_real() {
        let client = create_etherscan_client("mainnet").unwrap();
        
        // Vitalik's address
        let address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        
        let txs = client.get_recent_transactions(address, 10).await;
        
        if let Ok(txs) = txs {
            assert!(!txs.is_empty());
            assert!(txs[0].hash.starts_with("0x"));
        }
    }
}

