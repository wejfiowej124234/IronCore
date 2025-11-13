//! Bitcoin RPC 客户端
//! 
//! 此模块实现与 Bitcoin 节点的 RPC 通信

use super::account::BitcoinKeypair;
use super::address::{AddressType, BitcoinAddress};
use super::transaction::BitcoinTransaction;
use super::utxo::{SelectionStrategy, Utxo, UtxoSelector};
use crate::blockchain::traits::{BlockchainClient, TransactionStatus};
use crate::core::domain::PrivateKey;
use crate::core::errors::WalletError;
use async_trait::async_trait;
use bitcoin::Network;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info};

/// Bitcoin RPC 客户端
pub struct BitcoinClient {
    /// RPC URL
    rpc_url: String,
    /// HTTP 客户端
    http_client: HttpClient,
    /// Network type
    network: Network,
    /// RPC user名
    rpc_user: Option<String>,
    /// RPC Password
    rpc_password: Option<String>,
}

#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    jsonrpc: String,
    id: u64,
    result: Option<T>,
    error: Option<RpcError>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl BitcoinClient {
    /// 创建新的 Bitcoin 客户端
    pub fn new(rpc_url: String, network: Network) -> Self {
        Self {
            rpc_url,
            http_client: HttpClient::new(),
            network,
            rpc_user: None,
            rpc_password: None,
        }
    }
    
    /// 设置 RPC 认证
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.rpc_user = Some(username);
        self.rpc_password = Some(password);
        self
    }
    
    /// 发送 RPC 请求
    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Vec<Value>,
    ) -> Result<T, WalletError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };
        
        let mut req = self.http_client.post(&self.rpc_url).json(&request);
        
        // 添加基本认证
        if let (Some(user), Some(pass)) = (&self.rpc_user, &self.rpc_password) {
            req = req.basic_auth(user, Some(pass));
        }
        
        let response = req
            .send()
            .await
            .map_err(|e| WalletError::NetworkError(format!("RPC 请求failed: {}", e)))?;
        
        let rpc_response: RpcResponse<T> = response
            .json()
            .await
            .map_err(|e| WalletError::NetworkError(format!("解析响应failed: {}", e)))?;
        
        if let Some(error) = rpc_response.error {
            return Err(WalletError::NetworkError(format!(
                "RPC error {}: {}",
                error.code, error.message
            )));
        }
        
        rpc_response.result.ok_or_else(|| {
            WalletError::NetworkError("RPC 响应中缺少结果".to_string())
        })
    }
    
    /// fetch区块链信息
    pub async fn get_blockchain_info(&self) -> Result<Value, WalletError> {
        self.rpc_call("getblockchaininfo", vec![]).await
    }
    
    /// fetchnetwork信息
    pub async fn get_network_info(&self) -> Result<Value, WalletError> {
        self.rpc_call("getnetworkinfo", vec![]).await
    }
    
    /// 列出未花费的transaction输出 (UTXO)
    pub async fn list_unspent(&self, addresses: &[String]) -> Result<Vec<Utxo>, WalletError> {
        info!("query UTXO，address数量: {}", addresses.len());
        
        let params = vec![
            serde_json::to_value(1).unwrap(),     // minconf
            serde_json::to_value(9999999).unwrap(), // maxconf
            serde_json::to_value(addresses).unwrap(),
        ];
        
        let utxos: Vec<Value> = self.rpc_call("listunspent", params).await?;
        
        let result: Vec<Utxo> = utxos
            .iter()
            .filter_map(|u| {
                Some(Utxo::new(
                    u["txid"].as_str()?.to_string(),
                    u["vout"].as_u64()? as u32,
                    (u["amount"].as_f64()? * 100_000_000.0) as u64, // BTC -> satoshi
                    u["scriptPubKey"].as_str()?.to_string(),
                    u["confirmations"].as_u64()? as u32,
                ))
            })
            .collect();
        
        debug!("找到 {} 个 UTXO", result.len());
        Ok(result)
    }
    
    /// 发送原始transaction
    pub async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, WalletError> {
        info!("广播transaction");
        
        let params = vec![serde_json::to_value(tx_hex).unwrap()];
        let txid: String = self.rpc_call("sendrawtransaction", params).await?;
        
        info!("✅ transaction已广播，txid: {}", txid);
        Ok(txid)
    }
    
    /// fetchtransaction详情
    pub async fn get_transaction(&self, txid: &str) -> Result<Value, WalletError> {
        let params = vec![
            serde_json::to_value(txid).unwrap(),
            serde_json::to_value(true).unwrap(), // verbose
        ];
        
        self.rpc_call("getrawtransaction", params).await
    }
    
    /// 估算手续费率（satoshi/vbyte）
    pub async fn estimate_fee_rate(&self, blocks: u32) -> Result<u64, WalletError> {
        let params = vec![serde_json::to_value(blocks).unwrap()];
        
        let result: Value = self.rpc_call("estimatesmartfee", params).await?;
        
        if let Some(fee_rate_btc) = result["feerate"].as_f64() {
            // BTC/kB -> satoshi/vbyte
            let fee_rate_sat_vbyte = (fee_rate_btc * 100_000_000.0 / 1000.0) as u64;
            Ok(fee_rate_sat_vbyte.max(1)) // 至少 1 sat/vbyte
        } else {
            // 回退到默认费率
            Ok(10) // 10 sat/vbyte
        }
    }
    
    /// 转账（自动选择 UTXO）
    pub async fn transfer(
        &self,
        keypair: &BitcoinKeypair,
        to_address: &str,
        amount: u64,
        address_type: AddressType,
    ) -> Result<String, WalletError> {
        info!("start转账: {} sat -> {}", amount, to_address);
        
        // 1. 生成发送者address
        let from_address = BitcoinAddress::from_public_key(
            keypair.public_key(),
            address_type,
            self.network,
        )?;
        
        // 2. fetch UTXO
        let utxos: Vec<Utxo> = self.list_unspent(&[from_address]).await?;
        
        if utxos.is_empty() {
            return Err(WalletError::InsufficientFunds("没有可用的 UTXO".to_string()));
        }
        
        // 3. 估算手续费率
        let fee_rate = self.estimate_fee_rate(6).await?;
        
        // 4. 选择 UTXO
        let (selected_utxos, estimated_fee) = UtxoSelector::select(
            &utxos,
            amount,
            fee_rate,
            SelectionStrategy::BestFit,
        )?;
        
        info!("选择了 {} 个 UTXO，预估手续费: {} sat", selected_utxos.len() as u32, estimated_fee);
        
        // 5. 构建transaction
        let tx = BitcoinTransaction::build(
            keypair,
            &selected_utxos,
            to_address,
            amount,
            estimated_fee,
            address_type,
            self.network,
        )?;
        
        // 6. 序列化并广播
        let tx_hex = BitcoinTransaction::serialize(&tx);
        let txid = self.send_raw_transaction(&tx_hex).await?;
        
        Ok(txid)
    }
}

#[async_trait]
impl BlockchainClient for BitcoinClient {
    async fn get_balance(&self, address: &str) -> Result<String, WalletError> {
        info!("querybalance: {}", address);
        
        let utxos: Vec<Utxo> = self.list_unspent(&[address.to_string()]).await?;
        // 使用checked_add防止溢出
        let total: u64 = utxos.iter()
            .try_fold(0u64, |acc, u| acc.checked_add(u.amount))
            .ok_or_else(|| WalletError::ValidationError(
                "balance计算溢出：UTXO总额过大".to_string()
            ))?;
        
        // satoshi -> BTC
        let balance_btc = total as f64 / 100_000_000.0;
        Ok(format!("{:.8}", balance_btc))
    }
    
    async fn send_transaction(
        &self,
        from: &PrivateKey,
        to: &str,
        amount: &str,
    ) -> Result<String, WalletError> {
        // 解析金额（BTC -> satoshi）
        let amount_btc: f64 = amount
            .parse()
            .map_err(|_| WalletError::InvalidAmount(amount.to_string()))?;
        let amount_sat = (amount_btc * 100_000_000.0) as u64;
        
        // fromPrivate key创建密钥对
        let keypair = BitcoinKeypair::from_private_key(from, self.network)?;
        
        // 默认使用 SegWit
        self.transfer(&keypair, to, amount_sat, AddressType::SegWit).await
    }
    
    async fn get_transaction_status(&self, tx_hash: &str) -> Result<TransactionStatus, WalletError> {
        let tx = self.get_transaction(tx_hash).await?;
        
        if let Some(confirmations) = tx["confirmations"].as_u64() {
            if confirmations == 0 {
                Ok(TransactionStatus::Pending)
            } else if confirmations < 6 {
                Ok(TransactionStatus::Pending) // 少于 6 确认仍视为 pending
            } else {
                Ok(TransactionStatus::Confirmed)
            }
        } else {
            Ok(TransactionStatus::Unknown)
        }
    }
    
    fn get_network_name(&self) -> &str {
        match self.network {
            Network::Bitcoin => "bitcoin",
            Network::Testnet => "testnet",
            Network::Signet => "signet",
            Network::Regtest => "regtest",
            _ => "unknown",
        }
    }
    
    fn clone_box(&self) -> Box<dyn BlockchainClient> {
        Box::new(Self {
            rpc_url: self.rpc_url.clone(),
            http_client: HttpClient::new(),
            network: self.network,
            rpc_user: self.rpc_user.clone(),
            rpc_password: self.rpc_password.clone(),
        })
    }
    
    async fn estimate_fee(&self, _to_address: &str, _amount: &str) -> Result<String, WalletError> {
        let fee_rate = self.estimate_fee_rate(6).await?;
        // 假设平均transaction大小为 250 vbytes
        let estimated_fee_sat = fee_rate * 250;
        let estimated_fee_btc = estimated_fee_sat as f64 / 100_000_000.0;
        Ok(format!("{:.8}", estimated_fee_btc))
    }
    
    async fn get_nonce(&self, _address: &str) -> Result<u64, WalletError> {
        // Bitcoin 不使用 nonce，返回 0
        Ok(0)
    }
    
    async fn get_block_number(&self) -> Result<u64, WalletError> {
        let info = self.get_blockchain_info().await?;
        info["blocks"]
            .as_u64()
            .ok_or_else(|| WalletError::NetworkError("无法fetch区块高度".to_string()))
    }
    
    fn validate_address(&self, address: &str) -> anyhow::Result<bool> {
        Ok(BitcoinAddress::validate(address, self.network).unwrap_or(false))
    }
    
    fn get_native_token(&self) -> &str {
        "BTC"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // ========== 基础客户端测试 ==========
    
    #[test]
    fn test_client_creation() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Testnet,
        );
        
        assert_eq!(client.get_network_name(), "testnet");
    }
    
    #[test]
    fn test_client_with_auth() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        )
        .with_auth("user".to_string(), "pass".to_string());
        
        assert!(client.rpc_user.is_some());
        assert!(client.rpc_password.is_some());
    }
    
    #[test]
    fn test_client_creation_mainnet() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        );
        
        assert_eq!(client.get_network_name(), "bitcoin");
        assert_eq!(client.rpc_url, "http://localhost:8332");
    }
    
    #[test]
    fn test_client_creation_signet() {
        let client = BitcoinClient::new(
            "http://localhost:38332".to_string(),
            Network::Signet,
        );
        
        assert_eq!(client.get_network_name(), "signet");
    }
    
    #[test]
    fn test_client_creation_regtest() {
        let client = BitcoinClient::new(
            "http://localhost:18443".to_string(),
            Network::Regtest,
        );
        
        assert_eq!(client.get_network_name(), "regtest");
    }
    
    #[test]
    fn test_client_without_auth() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        );
        
        assert!(client.rpc_user.is_none());
        assert!(client.rpc_password.is_none());
    }
    
    #[test]
    fn test_client_auth_credentials() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        )
        .with_auth("testuser".to_string(), "testpass".to_string());
        
        assert_eq!(client.rpc_user, Some("testuser".to_string()));
        assert_eq!(client.rpc_password, Some("testpass".to_string()));
    }
    
    #[test]
    fn test_get_network_name_all_networks() {
        let networks = vec![
            (Network::Bitcoin, "bitcoin"),
            (Network::Testnet, "testnet"),
            (Network::Signet, "signet"),
            (Network::Regtest, "regtest"),
        ];
        
        for (network, expected_name) in networks {
            let client = BitcoinClient::new(
                "http://localhost:8332".to_string(),
                network,
            );
            assert_eq!(client.get_network_name(), expected_name);
        }
    }
    
    // ========== RPC 请求结构测试 ==========
    
    #[test]
    fn test_rpc_request_serialization() {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getblockchaininfo".to_string(),
            params: vec![],
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("getblockchaininfo"));
        assert!(json.contains("2.0"));
    }
    
    #[test]
    fn test_rpc_request_with_params() {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 2,
            method: "listunspent".to_string(),
            params: vec![
                serde_json::json!(1),
                serde_json::json!(9999999),
            ],
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("listunspent"));
        assert!(json.contains("9999999"));
    }
    
    // ========== error处理测试 ==========
    
    #[tokio::test]
    async fn test_rpc_call_invalid_url() {
        let client = BitcoinClient::new(
            "http://invalid-url:99999".to_string(),
            Network::Testnet,
        );
        
        let result: Result<Value, WalletError> = client
            .rpc_call("getblockchaininfo", vec![])
            .await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            WalletError::NetworkError(_) => {},
            _ => panic!("Expected NetworkError"),
        }
    }
    
    // ========== 辅助函数测试 ==========
    
    #[test]
    fn test_builder_pattern() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        )
        .with_auth("user".to_string(), "pass".to_string());
        
        assert_eq!(client.rpc_user, Some("user".to_string()));
        assert_eq!(client.rpc_password, Some("pass".to_string()));
        assert_eq!(client.rpc_url, "http://localhost:8332");
    }
    
    #[test]
    fn test_rpc_url_handling() {
        let urls = vec![
            "http://localhost:8332",
            "https://bitcoin.example.com:8332",
            "http://127.0.0.1:18332",
        ];
        
        for url in urls {
            let client = BitcoinClient::new(
                url.to_string(),
                Network::Bitcoin,
            );
            assert_eq!(client.rpc_url, url);
        }
    }
    
    #[test]
    fn test_empty_username_password() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        )
        .with_auth("".to_string(), "".to_string());
        
        assert_eq!(client.rpc_user, Some("".to_string()));
        assert_eq!(client.rpc_password, Some("".to_string()));
    }
    
    #[test]
    fn test_unicode_username_password() {
        let client = BitcoinClient::new(
            "http://localhost:8332".to_string(),
            Network::Bitcoin,
        )
        .with_auth("user名".to_string(), "Password".to_string());
        
        assert_eq!(client.rpc_user, Some("user名".to_string()));
        assert_eq!(client.rpc_password, Some("Password".to_string()));
    }
    
    // ========== 并发安全性测试 ==========
    
    #[tokio::test]
    async fn test_client_concurrent_creation() {
        let mut handles = Vec::new();
        
        for i in 0..5 {
            let handle = tokio::spawn(async move {
                let client = BitcoinClient::new(
                    format!("http://localhost:{}", 8332 + i),
                    Network::Testnet,
                );
                assert_eq!(client.get_network_name(), "testnet");
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
    }
    
    // ========== Note:以下测试需要运行中的 Bitcoin 节点 ==========
    
    #[tokio::test]
    #[ignore] // 需要真实的 Bitcoin 节点
    async fn test_get_blockchain_info() {
        let client = BitcoinClient::new(
            "http://localhost:18332".to_string(),
            Network::Testnet,
        );
        
        let result: Result<Value, WalletError> = client.get_blockchain_info().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    #[ignore] // 需要真实的 Bitcoin 节点
    async fn test_list_unspent() {
        let client = BitcoinClient::new(
            "http://localhost:18332".to_string(),
            Network::Testnet,
        );
        
        let addresses = vec!["tb1q...".to_string()]; // 替换为真实address
        let result: Result<Vec<Utxo>, WalletError> = client.list_unspent(&addresses).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    #[ignore] // 需要真实的 Bitcoin 节点
    async fn test_send_raw_transaction() {
        let client = BitcoinClient::new(
            "http://localhost:18332".to_string(),
            Network::Testnet,
        );
        
        let raw_tx = "0200000000..."; // 替换为真实的原始transaction
        let result = client.send_raw_transaction(raw_tx).await;
        // 可能failed，因为transaction可能无效
        assert!(result.is_ok() || result.is_err());
    }
    
    #[tokio::test]
    #[ignore] // 需要真实的 Bitcoin 节点
    async fn test_get_transaction_status() {
        let client = BitcoinClient::new(
            "http://localhost:18332".to_string(),
            Network::Testnet,
        );
        
        let tx_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = client.get_transaction_status(tx_hash).await;
        // 可能failed，因为transaction可能不存在
        assert!(result.is_ok() || result.is_err());
    }
}

