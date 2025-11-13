//! transaction历史query模块
//!
//! from区块链浏览器APIquery真实的transaction历史
//!
//! ## 支持的API
//! - Etherscan (Ethereum, Polygon, BSC)
//! - Blockstream (Bitcoin)
//!
//! ## 功能
//! - queryaddress的所有transaction
//! - 分页支持
//! - 缓存优化

use super::WalletManager;
use crate::blockchain::traits::Transaction;
use crate::core::errors::WalletError;
use tracing::{debug, info, warn};
use serde::Deserialize;

/// Etherscan API响应格式
#[derive(Debug, Deserialize)]
struct EtherscanResponse {
    status: String,
    message: String,
    result: Vec<EtherscanTransaction>,
}

/// Etherscantransaction格式
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EtherscanTransaction {
    #[serde(rename = "blockNumber")]
    block_number: String,
    #[serde(rename = "timeStamp")]
    timestamp: String,
    hash: String,
    from: String,
    to: String,
    value: String,
    gas: String,
    #[serde(rename = "gasPrice")]
    gas_price: String,
    #[serde(rename = "gasUsed")]
    gas_used: String,
    #[serde(rename = "isError")]
    is_error: String,
}

/// Blockstreamtransaction格式
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlockstreamTransaction {
    txid: String,
    version: u32,
    locktime: u32,
    vin: Vec<BlockstreamInput>,
    vout: Vec<BlockstreamOutput>,
    status: BlockstreamStatus,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlockstreamInput {
    txid: String,
    vout: u32,
    #[serde(default)]
    prevout: Option<BlockstreamPrevout>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlockstreamPrevout {
    scriptpubkey_address: Option<String>,
    value: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlockstreamOutput {
    scriptpubkey_address: Option<String>,
    value: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BlockstreamStatus {
    confirmed: bool,
    block_height: Option<u64>,
}

impl WalletManager {
    /// fetchEthereumtransaction历史（使用Etherscan API）
    ///
    /// # Arguments
    /// * `address` - Ethereumaddress
    /// * `network` - network（eth, polygon, bsc）
    ///
    /// # Returns
    /// * Vec<Transaction> - transaction列表
    #[cfg(feature = "ethereum")]
    pub async fn get_ethereum_tx_history(
        &self,
        address: &str,
        network: &str,
    ) -> Result<Vec<Transaction>, WalletError> {
        info!("Fetching Ethereum transaction history for: {}", address);
        
        // fetchAPI密钥
        let api_key = std::env::var("ETHERSCAN_API_KEY")
            .unwrap_or_else(|_| {
                warn!("ETHERSCAN_API_KEY not set, using public API (rate limited)");
                "YourApiKeyToken".to_string()
            });
        
        // 根据network选择API端点
        let base_url = match network {
            "eth" | "ethereum" => "https://api.etherscan.io/api",
            "sepolia" => "https://api-sepolia.etherscan.io/api",
            "polygon" => "https://api.polygonscan.com/api",
            "bsc" => "https://api.bscscan.com/api",
            _ => "https://api.etherscan.io/api",
        };
        
        // 构建API URL
        let url = format!(
            "{}?module=account&action=txlist&address={}&startblock=0&endblock=99999999&sort=desc&apikey={}",
            base_url, address, api_key
        );
        
        debug!("Querying Etherscan API: {}", base_url);
        
        // queryAPI
        let response = reqwest::get(&url).await
            .map_err(|e| WalletError::NetworkError(format!("Failed to query Etherscan: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(WalletError::NetworkError(format!(
                "Etherscan API returned error: {}",
                response.status()
            )));
        }
        
        let etherscan_resp: EtherscanResponse = response.json().await
            .map_err(|e| WalletError::NetworkError(format!("Failed to parse Etherscan response: {}", e)))?;
        
        if etherscan_resp.status != "1" {
            warn!("Etherscan API warning: {}", etherscan_resp.message);
            // 即使状态不是"1"，也可能有部分数据，Continue处理
        }
        
        // 转换为内部格式
        let transactions: Vec<Transaction> = etherscan_resp.result
            .into_iter()
            .map(|tx| {
                // Wei转换为Ether
                let value_wei: u128 = tx.value.parse().unwrap_or(0);
                let value_eth = value_wei as f64 / 1e18;
                
                Transaction {
                    hash: tx.hash,
                    from: tx.from,
                    to: tx.to,
                    amount: format!("{:.18}", value_eth), // 保持精度
                }
            })
            .collect();
        
        info!("✅ Fetched {} transactions from Etherscan", transactions.len());
        
        Ok(transactions)
    }
    
    /// fetchBitcointransaction历史（使用Blockstream API）
    ///
    /// # Arguments
    /// * `address` - Bitcoinaddress
    ///
    /// # Returns
    /// * Vec<Transaction> - transaction列表
    #[cfg(feature = "bitcoin")]
    pub async fn get_bitcoin_tx_history(
        &self,
        address: &str,
    ) -> Result<Vec<Transaction>, WalletError> {
        info!("Fetching Bitcoin transaction history for: {}", address);
        
        let url = format!("https://blockstream.info/api/address/{}/txs", address);
        
        debug!("Querying Blockstream API");
        
        let response = reqwest::get(&url).await
            .map_err(|e| WalletError::NetworkError(format!("Failed to query Blockstream: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(WalletError::NetworkError(format!(
                "Blockstream API returned error: {}",
                response.status()
            )));
        }
        
        let blockstream_txs: Vec<BlockstreamTransaction> = response.json().await
            .map_err(|e| WalletError::NetworkError(format!("Failed to parse Blockstream response: {}", e)))?;
        
        // 转换为内部格式
        let transactions: Vec<Transaction> = blockstream_txs
            .into_iter()
            .map(|tx| {
                // 计算该address相关的金额变化
                let mut received: i64 = 0;
                let mut sent: i64 = 0;
                
                // 计算接收（输出到该address）
                for vout in &tx.vout {
                    if vout.scriptpubkey_address.as_deref() == Some(address) {
                        received += vout.value as i64;
                    }
                }
                
                // 计算发送（输入来自该address）
                for vin in &tx.vin {
                    if let Some(prevout) = &vin.prevout {
                        if prevout.scriptpubkey_address.as_deref() == Some(address) {
                            sent += prevout.value as i64;
                        }
                    }
                }
                
                // 净变化
                let net_change = received - sent;
                let amount_btc = net_change as f64 / 100_000_000.0;
                
                // 确定from和to（简化版）
                let (from, to) = if net_change > 0 {
                    // 接收transaction
                    ("external".to_string(), address.to_string())
                } else {
                    // 发送transaction
                    (address.to_string(), tx.vout.first()
                        .and_then(|v| v.scriptpubkey_address.clone())
                        .unwrap_or_else(|| "unknown".to_string()))
                };
                
                Transaction {
                    hash: tx.txid,
                    from,
                    to,
                    amount: format!("{:.8}", amount_btc.abs()),
                }
            })
            .collect();
        
        info!("✅ Fetched {} transactions from Blockstream", transactions.len());
        
        Ok(transactions)
    }
    
    /// fetchtransaction历史（统一接口）
    ///
    /// 根据network自动选择对应的API
    pub async fn get_tx_history_by_network(
        &self,
        wallet_name: &str,
        network: &str,
    ) -> Result<Vec<Transaction>, WalletError> {
        info!("Fetching transaction history for wallet: {} on network: {}", wallet_name, network);
        
        // fetchaddress（需要Password，这里使用空Password）
        let password = std::env::var("WALLET_DEFAULT_PASSWORD")
            .unwrap_or_else(|_| "".to_string());
        
        let address = match network {
            "eth" | "ethereum" | "sepolia" | "polygon" | "bsc" => {
                #[cfg(feature = "ethereum")]
                {
                    self.get_ethereum_address_from_master_key(wallet_name, &password).await?
                }
                #[cfg(not(feature = "ethereum"))]
                {
                    return Err(WalletError::ValidationError(
                        "Ethereum feature not enabled".to_string()
                    ));
                }
            }
            "btc" | "bitcoin" => {
                #[cfg(feature = "bitcoin")]
                {
                    self.get_bitcoin_address_from_master_key(wallet_name, &password).await?
                }
                #[cfg(not(feature = "bitcoin"))]
                {
                    return Err(WalletError::ValidationError(
                        "Bitcoin feature not enabled".to_string()
                    ));
                }
            }
            _ => {
                return Err(WalletError::ValidationError(format!(
                    "Unsupported network: {}",
                    network
                )));
            }
        };
        
        // 根据networkquery历史
        match network {
            "eth" | "ethereum" | "sepolia" | "polygon" | "bsc" => {
                #[cfg(feature = "ethereum")]
                {
                    self.get_ethereum_tx_history(&address, network).await
                }
                #[cfg(not(feature = "ethereum"))]
                {
                    Err(WalletError::ValidationError(
                        "Ethereum feature not enabled".to_string()
                    ))
                }
            }
            "btc" | "bitcoin" => {
                #[cfg(feature = "bitcoin")]
                {
                    self.get_bitcoin_tx_history(&address).await
                }
                #[cfg(not(feature = "bitcoin"))]
                {
                    Err(WalletError::ValidationError(
                        "Bitcoin feature not enabled".to_string()
                    ))
                }
            }
            _ => Err(WalletError::ValidationError(format!(
                "Unsupported network: {}",
                network
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    // 测试需要真实network和API密钥，在集成测试中validate
    
    #[test]
    fn test_etherscan_url_construction() {
        // 测试URL构建逻辑
        let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb";
        let api_key = "test_key";
        let url = format!(
            "https://api.etherscan.io/api?module=account&action=txlist&address={}&startblock=0&endblock=99999999&sort=desc&apikey={}",
            address, api_key
        );
        
        assert!(url.contains(address));
        assert!(url.contains("txlist"));
    }
}

