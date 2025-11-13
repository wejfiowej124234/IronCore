//! 跨链桥接模块
//!
//! 提供跨链资产桥接功能

use super::WalletManager;
use crate::core::errors::WalletError;
use crate::blockchain::bridge::{BridgeTransaction, BridgeTransactionStatus};
use tracing::info;

impl WalletManager {
    /// 桥接资产
    ///
    /// # Arguments
    /// * `from_wallet` - 源Wallet name
    /// * `from_chain` - 源链
    /// * `to_chain` - 目标链
    /// * `token` - 代币符号
    /// * `amount` - 金额
    ///
    /// # Returns
    /// 桥接transaction ID
    pub async fn bridge_assets(
        &self,
        from_wallet: &str,
        from_chain: &str,
        to_chain: &str,
        token: &str,
        amount: &str,
    ) -> Result<String, WalletError> {
        info!(
            "Bridging {} {} from {} ({}) to {}",
            amount, token, from_wallet, from_chain, to_chain
        );

        // validatewallet存在
        let _wallet = self
            .get_wallet_by_name(from_wallet)
            .await?
            .ok_or_else(|| WalletError::NotFoundError(format!("Wallet not found: {}", from_wallet)))?;

        // 创建桥接transaction
        let bridge_tx = BridgeTransaction {
            id: uuid::Uuid::new_v4().to_string(),
            from_wallet: from_wallet.to_string(),
            from_chain: from_chain.to_string(),
            to_chain: to_chain.to_string(),
            token: token.to_string(),
            amount: amount.to_string(),
            status: BridgeTransactionStatus::Initiated,
            source_tx_hash: None,
            destination_tx_hash: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            fee_amount: None,
            estimated_completion_time: None,
        };

        let tx_id = bridge_tx.id.clone();

        // 存储桥接transaction
        {
            let mut transactions = self.bridge_transactions.write();
            transactions.insert(tx_id.clone(), bridge_tx);
        }

        info!("✅ Bridge transaction created: {}", tx_id);
        Ok(tx_id)
    }

    /// fetch区块高度（真实的RPCquery）
    /// 
    /// # Arguments
    /// * `network` - network名称（eth, sepolia, polygon, bsc等）
    /// 
    /// # Returns
    /// 当前区块高度
    /// 
    /// # 实现说明
    /// 使用ethers库通过RPCquery真实的区块高度。
    /// 这是一个标准的区块链query，不涉及Private key或敏感操作。
    pub async fn get_block_number(&self, network: &str) -> Result<u64, WalletError> {
        info!("Getting block number for network: {}", network);

        #[cfg(feature = "ethereum")]
        {
            use ethers::prelude::{Provider, Http, Middleware};
            
            // fetchRPC URL
            let rpc_url = self.config.blockchain.networks.get(network)
                .map(|n| n.rpc_url.as_str())
                .unwrap_or_else(|| match network {
                    "eth" | "ethereum" => "https://eth.llamarpc.com",
                    "sepolia" => "https://rpc.sepolia.org",
                    "polygon" => "https://polygon-rpc.com",
                    "bsc" => "https://bsc-dataseed.binance.org",
                    _ => "https://eth.llamarpc.com"
                });
            
            info!("query区块高度，RPC: {}", rpc_url);
            
            // 创建Provider
            let provider = Provider::<Http>::try_from(rpc_url)
                .map_err(|e| WalletError::NetworkError(format!("无法连接到RPC: {}", e)))?;
            
            // query当前区块高度
            let block_number = provider.get_block_number().await
                .map_err(|e| WalletError::NetworkError(format!("query区块高度failed: {}", e)))?;
            
            let height = block_number.as_u64();
            info!("✅ 当前区块高度: {} (network: {})", height, network);
            
            Ok(height)
        }
        
        #[cfg(not(feature = "ethereum"))]
        {
            Err(WalletError::NotImplemented(format!(
                "Network '{}' is not supported (ethereum feature disabled)", 
                network
            )))
        }
    }

    /// check桥接状态
    ///
    /// # Errors
    /// 返回`WalletError::NotFoundError`如果桥接transaction不存在
    pub async fn check_bridge_status(
        &self,
        tx_id: &str,
    ) -> Result<BridgeTransactionStatus, WalletError> {
        let transactions = self.bridge_transactions.read();
        let tx = transactions
            .get(tx_id)
            .ok_or_else(|| WalletError::NotFoundError(format!("Bridge transaction not found: {}", tx_id)))?;

        Ok(tx.status.clone())
    }

    /// fetch桥接transaction状态
    ///
    /// # Errors
    /// 返回`WalletError::NotFoundError`如果桥接transaction不存在
    pub async fn get_bridge_transaction_status(
        &self,
        tx_id: &str,
    ) -> Result<BridgeTransaction, WalletError> {
        let transactions = self.bridge_transactions.read();
        let tx = transactions
            .get(tx_id)
            .ok_or_else(|| WalletError::NotFoundError(format!("Bridge transaction not found: {}", tx_id)))?
            .clone();

        Ok(tx)
    }

    /// 更新桥接transaction状态
    ///
    /// # Errors
    /// 返回`WalletError::NotFoundError`如果桥接transaction不存在
    pub async fn update_bridge_transaction_status(
        &self,
        tx_id: &str,
        status: BridgeTransactionStatus,
    ) -> Result<(), WalletError> {
        let mut transactions = self.bridge_transactions.write();
        let tx = transactions
            .get_mut(tx_id)
            .ok_or_else(|| WalletError::NotFoundError(format!("Bridge transaction not found: {}", tx_id)))?;

        tx.status = status;
        Ok(())
    }

    /// 计算桥接费用
    /// 
    /// ⚠️ 当前实现：简化的固定费率模型
    /// 
    /// # 企业级跨链桥费用计算应该考虑：
    /// 1. **源链Gas费**: 锁定资产的transaction费用
    /// 2. **目标链Gas费**: 铸造/解锁资产的transaction费用  
    /// 3. **validate者费用**: 跨链validatenetwork的费用
    /// 4. **network拥堵**: 动态调整费率
    /// 5. **流动性深度**: 大额transaction可能需要更高费率
    /// 6. **汇率波动**: 跨链期间的价格风险补偿
    /// 
    /// # 推荐方案
    /// - 使用ChainLink喂价fetch实时Gas费
    /// - 参考主流跨链桥费率（Wormhole、LayerZero）
    /// - 实现动态费率调整机制
    pub fn calculate_bridge_fee(
        &self,
        from_chain: &str,
        to_chain: &str,
        amount: &str,
    ) -> String {
        // ⚠️ 简化的固定费率模型（仅用于演示）
        // 真实环境需要query链上Gas费和桥接协议费率
        
        let amount_f64 = amount.parse::<f64>().unwrap_or(0.0);
        
        // 基础费率：0.3%
        let base_rate = 0.003;
        
        // 简化的链特性调整（实际应query实时数据）
        let chain_factor = match (from_chain, to_chain) {
            ("eth", "polygon") | ("polygon", "eth") => 1.0,  // 低成本
            ("eth", "bsc") | ("bsc", "eth") => 1.2,          // 中等成本
            _ => 1.5,                                          // 其他链组合
        };
        
        let fee = amount_f64 * base_rate * chain_factor;
        
        info!("计算桥接费用: {} {} -> {} = {} (费率: {:.2}%)", 
              amount, from_chain, to_chain, fee, base_rate * chain_factor * 100.0);

        fee.to_string()
    }
}

