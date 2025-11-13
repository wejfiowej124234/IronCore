//! address派生模块
//!
//! 提供基于主密钥的address派生功能

use super::WalletManager;
use crate::core::errors::WalletError;
use tracing::debug;

impl WalletManager {
    /// from主密钥派生address
    ///
    /// # Arguments
    /// * `master_key` - 主密钥字节
    /// * `network` - network名称
    ///
    /// # Returns
    /// 派生的address字符串
    #[allow(unused_variables)] // master_key在没有启用区块链features时未使用
    pub fn derive_address(
        &self,
        master_key: &[u8],
        network: &str,
    ) -> Result<String, WalletError> {
        debug!("Deriving address for network: {}", network);

        match network {
            #[cfg(feature = "ethereum")]
            "eth" | "ethereum" | "sepolia" | "polygon" | "bsc" => {
                self.derive_ethereum_address(master_key)
            }
            // #[cfg(feature = "polygon")]
            // "polygon" | "polygon-testnet" => {
            // }
            #[cfg(feature = "bitcoin")]
            "bitcoin" | "btc" => {
                self.derive_bitcoin_address(master_key)
            }
            _ => Err(WalletError::ValidationError(format!(
                "Unsupported network for address derivation: {}",
                network
            ))),
        }
    }

    #[cfg(feature = "ethereum")]
    fn derive_ethereum_address(&self, master_key: &[u8]) -> Result<String, WalletError> {
        use secp256k1::{PublicKey, Secp256k1, SecretKey};
        use sha3::{Digest, Keccak256};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(master_key)
            .map_err(|e| WalletError::CryptoError(e.to_string()))?;
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);

        // Ethereum address = keccak256(pubkey)[12..]
        let pub_bytes = public_key.serialize_uncompressed();
        let hash = Keccak256::digest(&pub_bytes[1..]); // 跳过前缀字节
        let address = format!("0x{}", hex::encode(&hash[12..]));

        Ok(address)
    }


    #[cfg(feature = "bitcoin")]
    fn derive_bitcoin_address(&self, master_key: &[u8]) -> Result<String, WalletError> {
        use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey};
        use bitcoin::{Address, Network};
        use bitcoin::PublicKey as BitcoinPublicKey;
        
        if master_key.len() != 32 {
            return Err(WalletError::CryptoError(
                format!("Invalid master key length: expected 32, got {}", master_key.len())
            ));
        }
        
        // 创建secp256k1上下文
        let secp = Secp256k1::new();
        
        // frommaster_key创建Private key
        let secret_key = SecretKey::from_slice(master_key)
            .map_err(|e| WalletError::CryptoError(format!("Invalid secret key: {}", e)))?;
        
        // 生成公钥
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        
        // 转换为Bitcoin公钥类型
        let bitcoin_pubkey = BitcoinPublicKey::new(public_key);
        
        // 生成P2PKHaddress（以1开头的传统address）
        let address = Address::p2pkh(&bitcoin_pubkey, Network::Bitcoin);
        
        Ok(address.to_string())
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
    async fn test_derive_address_unsupported_network() {
        let manager = create_test_manager().await;
        let master_key = vec![1u8; 32];
        let result = manager.derive_address(&master_key, "unknown");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::ValidationError(_)));
    }
    
    #[cfg(feature = "ethereum")]
    #[tokio::test]
    async fn test_derive_ethereum_address() {
        let manager = create_test_manager().await;
        let master_key = vec![1u8; 32];
        let result = manager.derive_address(&master_key, "eth");
        assert!(result.is_ok());
        let address = result.unwrap();
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);
    }
    
    #[cfg(feature = "ethereum")]
    #[tokio::test]
    async fn test_derive_ethereum_address_deterministic() {
        let manager = create_test_manager().await;
        let master_key = vec![42u8; 32];
        let addr1 = manager.derive_address(&master_key, "eth").unwrap();
        let addr2 = manager.derive_address(&master_key, "eth").unwrap();
        assert_eq!(addr1, addr2);
    }
    
    #[cfg(feature = "ethereum")]
    #[tokio::test]
    async fn test_derive_ethereum_address_different_keys() {
        let manager = create_test_manager().await;
        let key1 = vec![1u8; 32];
        let key2 = vec![2u8; 32];
        let addr1 = manager.derive_address(&key1, "eth").unwrap();
        let addr2 = manager.derive_address(&key2, "eth").unwrap();
        assert_ne!(addr1, addr2);
    }
    
    
    #[tokio::test]
    async fn test_derive_address_empty_key() {
        let manager = create_test_manager().await;
        let empty_key = vec![];
        let result = manager.derive_address(&empty_key, "eth");
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_derive_address_short_key() {
        let manager = create_test_manager().await;
        let short_key = vec![1u8; 16];
        let result = manager.derive_address(&short_key, "eth");
        assert!(result.is_err());
    }
}

