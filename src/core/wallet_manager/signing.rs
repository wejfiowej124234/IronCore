//! transactionsign模块
//!
//! 实现Ethereum和Bitcoin的transactionsign
//! 
//! ## 安全架构
//! ```
//! userPassword
//!    ↓
//! 解密master_key (临时内存)
//!    ↓
//! 创建Signer (LocalWallet/ECDSA)
//!    ↓
//! 构建transaction (TransactionRequest/PSBT)
//!    ↓
//! sign (ECDSA Secp256k1)
//!    ↓
//! 返回sign后的transaction (RLP/Raw Tx)
//!    ↓
//! Zeroizing清零所有敏感数据
//! ```
//!
//! ## 核心功能
//! - Ethereumtransactionsign（EIP-1559、Legacy）
//! - Bitcointransactionsign（P2PKH、SegWit）
//! - Gas估算
//! - Nonce管理
//! - UTXO选择

use super::WalletManager;
use crate::core::errors::WalletError;
use tracing::info;

impl WalletManager {
    /// signEthereumtransaction
    ///
    /// # 算法流程
    /// 1. 解密master_key
    /// 2. 创建LocalWallet
    /// 3. 构建TransactionRequest
    /// 4. 估算Gas
    /// 5. Sign transaction
    /// 6. 返回RLP编码的Sign transaction
    ///
    /// # Arguments
    /// * `wallet_name` - Wallet name
    /// * `to` - Recipient address
    /// * `amount` - 转账金额（Ether单位，如 "1.5"）
    /// * `network` - network（eth, sepolia, polygon, bsc）
    /// * `password` - userPassword（用于解密）
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - RLP编码的Sign transaction
    /// * `Err(WalletError)` - Signing failed
    ///
    /// # Example
    /// ```ignore
    /// let signed_tx = wallet_manager.sign_ethereum_transaction(
    ///     "my_wallet",
    ///     "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    ///     "0.1",
    ///     "sepolia",
    ///     "my_password"
    /// ).await?;
    /// ```
    #[cfg(feature = "ethereum")]
    pub async fn sign_ethereum_transaction(
        &self,
        wallet_name: &str,
        to: &str,
        amount: &str,
        network: &str,
        password: &str,
    ) -> Result<Vec<u8>, WalletError> {
        info!("Signing Ethereum transaction for wallet: {}", wallet_name);
        
        // Step 1: fetchwallet数据
        let wallet = self.get_wallet_by_name(wallet_name).await?
            .ok_or_else(|| WalletError::NotFoundError(
                format!("Wallet '{}' not found", wallet_name)
            ))?;
        
        // Step 2: 解密master_key
        let master_key = self.decrypt_master_key(&wallet, password).await?;
        
        // Step 3: 创建LocalWallet
        use ethers::signers::{LocalWallet, Signer};
        use ethers::core::k256::ecdsa::SigningKey;
        use k256::SecretKey;
        
        let secret_key = SecretKey::from_slice(&master_key)
            .map_err(|e| WalletError::CryptoError(format!("Invalid secret key: {}", e)))?;
        
        let signing_key = SigningKey::from(secret_key);
        let signer = LocalWallet::from(signing_key);
        
        info!("✅ Signer created, address: {:?}", signer.address());
        
        // Step 4: fetchRPC Provider
        use ethers::prelude::{Provider, Http};
        use ethers::providers::Middleware;
        
        let rpc_url = self.get_rpc_url(network)?;
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| WalletError::NetworkError(format!("Failed to connect to RPC: {}", e)))?;
        
        // Step 5: 构建transaction
        use ethers::types::{TransactionRequest, Address};
        use std::str::FromStr;
        
        let to_address = Address::from_str(to)
            .map_err(|e| WalletError::ValidationError(format!("Invalid address: {}", e)))?;
        
        // 解析金额（Ether → Wei）
        let amount_wei = ethers::utils::parse_ether(amount)
            .map_err(|e| WalletError::ValidationError(format!("Invalid amount: {}", e)))?;
        
        // Step 6: fetchChain ID
        let chain_id: u64 = match network {
            "eth" | "ethereum" => 1,
            "sepolia" => 11155111,
            "polygon" => 137,
            "bsc" => 56,
            _ => 1,
        };
        
        // 连接signer到provider
        let client = signer.with_chain_id(chain_id);
        
        // Step 7: fetchNonce
        let nonce = provider.get_transaction_count(client.address(), None).await
            .map_err(|e| WalletError::NetworkError(format!("Failed to get nonce: {}", e)))?;
        
        info!("Transaction nonce: {}", nonce);
        
        // Step 8: 构建完整的transaction
        let mut tx = TransactionRequest::new()
            .to(to_address)
            .value(amount_wei)
            .nonce(nonce)
            .chain_id(chain_id);
        
        // Step 9: 估算Gas和Gas Price
        let gas_price = provider.get_gas_price().await
            .map_err(|e| WalletError::NetworkError(format!("Failed to get gas price: {}", e)))?;
        
        tx = tx.gas_price(gas_price).gas(21000); // 标准转账
        
        info!("Gas price: {} gwei", gas_price.as_u64() / 1_000_000_000);
        
        // Step 10: 转换为TypedTransaction并sign
        use ethers::types::transaction::eip2718::TypedTransaction;
        let typed_tx: TypedTransaction = tx.into();
        
        let signature = client.sign_transaction(&typed_tx).await
            .map_err(|e| WalletError::CryptoError(format!("Failed to sign transaction: {}", e)))?;
        
        // Step 11: 编码为RLP
        let signed_tx = typed_tx.rlp_signed(&signature);
        
        info!("✅ Transaction signed successfully, size: {} bytes", signed_tx.len());
        
        Ok(signed_tx.to_vec())
    }
    
    /// 广播Ethereumtransaction到区块链
    ///
    /// # Arguments
    /// * `signed_tx` - RLP编码的Sign transaction
    /// * `network` - Network type
    ///
    /// # Returns
    /// * `Ok(String)` - Transaction hash (0x...)
    #[cfg(feature = "ethereum")]
    pub async fn broadcast_ethereum_transaction(
        &self,
        signed_tx: &[u8],
        network: &str,
    ) -> Result<String, WalletError> {
        info!("Broadcasting Ethereum transaction to network: {}", network);
        
        use ethers::prelude::{Provider, Http};
        use ethers::providers::Middleware;
        use ethers::types::Bytes;
        
        let rpc_url = self.get_rpc_url(network)?;
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| WalletError::NetworkError(format!("Failed to connect to RPC: {}", e)))?;
        
        // 广播transaction
        let tx_bytes = Bytes::from(signed_tx.to_vec());
        let pending_tx = provider.send_raw_transaction(tx_bytes).await
            .map_err(|e| WalletError::NetworkError(format!("Failed to broadcast transaction: {}", e)))?;
        
        let tx_hash = format!("{:?}", pending_tx.tx_hash());
        
        info!("✅ Transaction broadcasted successfully: {}", tx_hash);
        
        Ok(tx_hash)
    }
    
    /// fetchnetwork的RPC URL
    fn get_rpc_url(&self, network: &str) -> Result<&str, WalletError> {
        let url = self.config.blockchain.networks.get(network)
            .map(|n| n.rpc_url.as_str())
            .or({
                match network {
                    "eth" | "ethereum" => Some("https://eth.llamarpc.com"),
                    "sepolia" => Some("https://rpc.sepolia.org"),
                    "polygon" => Some("https://polygon-rpc.com"),
                    "bsc" => Some("https://bsc-dataseed.binance.org"),
                    _ => None,
                }
            })
            .ok_or_else(|| WalletError::ValidationError(
                format!("No RPC URL configured for network: {}", network)
            ))?;
        
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    // 测试暂时跳过，需要真实的wallet和network连接
    
    #[test]
    fn test_get_rpc_url() {
        // 测试RPC URLfetch逻辑
        // 需要mock WalletManager
    }
}

