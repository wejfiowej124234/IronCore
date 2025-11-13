//! balancequery模块
//!
//! 提供walletbalancequery功能

use super::WalletManager;
use crate::core::errors::WalletError;
use tracing::info;



impl WalletManager {
    /// querywalletbalance
    ///
    /// # Arguments
    /// * `wallet_name` - Wallet name
    /// * `network` - network名称
    /// * `password` - walletPassword（用于解密主密钥）
    ///
    /// # Returns
    /// balance字符串
    pub async fn get_balance(
        &self,
        wallet_name: &str,
        network: &str,
        password: &str,  // ✅ 添加Password参数
    ) -> Result<String, WalletError> {
        info!("Getting balance for wallet: {} on network: {}", wallet_name, network);

        // fetchwallet
        let _wallet = self
            .get_wallet_by_name(wallet_name)
            .await?
            .ok_or_else(|| WalletError::NotFoundError(format!("Wallet '{}' not found", wallet_name)))?;

        // 根据networkquerybalance
        match network {
            #[cfg(feature = "ethereum")]
            "eth" | "ethereum" | "sepolia" | "polygon" | "bsc" => {
                self.get_ethereum_balance(wallet_name, network, password).await  // ✅ 传递Password
            }
            // ✅ 支持Bitcoinnetwork
            "btc" | "bitcoin" => {
                self.get_bitcoin_balance(wallet_name, network, password).await  // ✅ 传递Password
            }
            _ => Err(WalletError::ValidationError(format!(
                "Unsupported network: {}. Currently supported: eth, btc, polygon, bsc.",
                network
            ))),
        }
    }

    /// fetchEthereumbalance（真实实现）
    #[cfg(feature = "ethereum")]
    async fn get_ethereum_balance(
        &self,
        wallet_name: &str,
        network: &str,
        password: &str,  // ✅ 添加Password参数
    ) -> Result<String, WalletError> {
        use ethers::prelude::{Provider, Http};
        
        // fetchwallet（validate存在性）
        let _wallet = self.get_wallet_by_name(wallet_name).await?
            .ok_or_else(|| WalletError::NotFoundError(format!("Wallet '{}' not found", wallet_name)))?;
        
        // fetchnetworkRPC URL
        let rpc_url = self.config.blockchain.networks.get(network)
            .map(|n| n.rpc_url.as_str())
            .unwrap_or_else(|| match network {
                "eth" | "ethereum" => "https://eth.llamarpc.com",
                "sepolia" => "https://rpc.sepolia.org",
                "polygon" => "https://polygon-rpc.com",
                "bsc" => "https://bsc-dataseed.binance.org",
                _ => "https://eth.llamarpc.com"
            });
        
        info!("querybalance: wallet={}, network={}, rpc={}", wallet_name, network, rpc_url);
        
        // 创建Provider
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| WalletError::NetworkError(format!("无法连接到RPC: {}", e)))?;
        
        // ✅ frommaster_key推导address（核心算法）
        // ✅ 使用传入的Password（from会话fetch）
        let address = self.get_ethereum_address_from_master_key(wallet_name, password).await?;
        
        info!("Querying balance for address: {}", address);
        
        // ✅ query真实balance
        use ethers::types::Address as EthAddress;
        use std::str::FromStr;
        
        let eth_address = EthAddress::from_str(&address)
            .map_err(|e| WalletError::ValidationError(format!("Invalid address: {}", e)))?;
        
        // 使用ethers 2.0的API
        use ethers::providers::Middleware;
        let balance = provider.get_balance(eth_address, None).await
            .map_err(|e| WalletError::NetworkError(format!("Failed to query balance: {}", e)))?;
        
        // 转换为字符串（以Ether为单位）
        let balance_eth = ethers::utils::format_units(balance, "ether")
            .map_err(|e| WalletError::ValidationError(format!("Failed to format balance: {}", e)))?;
        
        info!("✅ Real balance for {}: {} ETH", address, balance_eth);
        Ok(balance_eth)
    }

    /// fetchBitcoinbalance
    async fn get_bitcoin_balance(
        &self,
        wallet_name: &str,
        _network: &str,
        password: &str,  // ✅ 添加Password参数
    ) -> Result<String, WalletError> {
        info!("Getting Bitcoin balance for wallet: {}", wallet_name);
        
        // fetchwallet（validate存在性）
        let _wallet = self.get_wallet_by_name(wallet_name).await?
            .ok_or_else(|| WalletError::NotFoundError(format!("Wallet '{}' not found", wallet_name)))?;
        
        // ✅ 使用传入的Password（from会话fetch）
        let address = self.get_bitcoin_address_from_master_key(wallet_name, password).await?;
        
        info!("Querying Bitcoin balance for address: {}", address);
        
        // ✅ 使用Blockstream APIquery真实balance
        let url = format!("https://blockstream.info/api/address/{}", address);
        
        #[derive(serde::Deserialize)]
        struct AddressInfo {
            chain_stats: ChainStats,
        }
        
        #[derive(serde::Deserialize)]
        struct ChainStats {
            funded_txo_sum: u64,
            spent_txo_sum: u64,
        }
        
        let response = reqwest::get(&url).await
            .map_err(|e| WalletError::NetworkError(format!("Failed to connect to Blockstream API: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(WalletError::NetworkError(format!(
                "Blockstream API returned error: {}",
                response.status()
            )));
        }
        
        let addr_info: AddressInfo = response.json().await
            .map_err(|e| WalletError::NetworkError(format!("Failed to parse Blockstream response: {}", e)))?;
        
        // 计算balance（satoshi）
        let balance_satoshi = addr_info.chain_stats.funded_txo_sum
            .saturating_sub(addr_info.chain_stats.spent_txo_sum);
        
        // 转换为BTC（1 BTC = 100,000,000 satoshi）
        let balance_btc = balance_satoshi as f64 / 100_000_000.0;
        let balance_str = format!("{:.8}", balance_btc);
        
        info!("✅ Real Bitcoin balance for {}: {} BTC ({} satoshi)", 
              address, balance_str, balance_satoshi);
        
        Ok(balance_str)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::WalletConfig;

    async fn create_test_manager() -> WalletManager {
        std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        std::env::set_var("TEST_SKIP_DECRYPT", "1");
        let config = WalletConfig::default();
        WalletManager::new(&config).await.unwrap()
    }

    #[tokio::test]
    async fn test_get_balance_wallet_not_found() {
        let manager = create_test_manager().await;
        let result = manager.get_balance("nonexistent", "eth", "test_password").await;  // ✅ 添加Password
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::NotFoundError(_)));
    }
    
    #[tokio::test]
    async fn test_get_balance_unsupported_network() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("test", "test_password", false).await;
        let result = manager.get_balance("test", "unsupported_chain", "test_password").await;  // ✅ 添加Password
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::ValidationError(_)));
    }
    
    #[tokio::test]
    async fn test_get_balance_empty_wallet_name() {
        let manager = create_test_manager().await;
        let result = manager.get_balance("", "eth", "test_password").await;  // ✅ 添加Password
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_get_balance_empty_network() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("test", "test_password", false).await;
        let result = manager.get_balance("test", "", "test_password").await;  // ✅ 添加Password
        assert!(result.is_err());
    }
}


