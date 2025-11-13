//! Nonce 管理模块
//!
//! 提供transaction nonce 的追踪和管理功能

use super::WalletManager;
use crate::core::errors::WalletError;
use tracing::{debug, info};

impl WalletManager {
    /// fetch下一个 nonce
    ///
    /// # Arguments
    /// * `address` - walletaddress
    /// * `network` - network名称
    ///
    /// # Returns
    /// 下一个可用的 nonce
    ///
    /// # Errors
    /// 返回`WalletError::ValidationError`如果nonce溢出
    pub async fn get_next_nonce(
        &self,
        address: &str,
        network: &str,
    ) -> Result<u64, WalletError> {
        debug!("Getting next nonce for {} on {}", address, network);

        let mut tracker = self.nonce_tracker.write();
        let network_map = tracker
            .entry(address.to_string())
            .or_default();

        let nonce = network_map.entry(network.to_string()).or_insert(0);
        let next_nonce = *nonce;
        
        // ✅ checknonce溢出
        *nonce = nonce.checked_add(1).ok_or_else(|| {
            WalletError::ValidationError(format!(
                "Nonce overflow for {} on {}", address, network
            ))
        })?;

        info!("Next nonce for {} on {}: {}", address, network, next_nonce);
        Ok(next_nonce)
    }

    /// 标记 nonce 已使用
    ///
    /// # Arguments
    /// * `address` - walletaddress
    /// * `network` - network名称
    /// * `nonce` - 已使用的 nonce
    ///
    /// # Errors
    /// 返回`WalletError::ValidationError`如果nonce溢出
    pub async fn mark_nonce_used(
        &self,
        address: &str,
        network: &str,
        nonce: u64,
    ) -> Result<(), WalletError> {
        debug!("Marking nonce {} as used for {} on {}", nonce, address, network);

        let mut tracker = self.nonce_tracker.write();
        let network_map = tracker
            .entry(address.to_string())
            .or_default();

        let current = network_map.entry(network.to_string()).or_insert(0);
        if nonce >= *current {
            // ✅ check溢出
            *current = nonce.checked_add(1).ok_or_else(|| {
                WalletError::ValidationError(format!(
                    "Nonce overflow in mark_nonce_used for {} on {}", address, network
                ))
            })?;
        }

        Ok(())
    }

    /// 重置 nonce
    ///
    /// # Arguments
    /// * `address` - walletaddress
    /// * `network` - network名称
    pub async fn reset_nonce(
        &self,
        address: &str,
        network: &str,
    ) -> Result<(), WalletError> {
        info!("Resetting nonce for {} on {}", address, network);

        let mut tracker = self.nonce_tracker.write();
        if let Some(network_map) = tracker.get_mut(address) {
            network_map.insert(network.to_string(), 0);
        }

        Ok(())
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
    async fn test_get_next_nonce_first_time() {
        let manager = create_test_manager().await;
        let nonce = manager.get_next_nonce("0x1234", "eth").await.unwrap();
        assert_eq!(nonce, 0);
    }
    
    #[tokio::test]
    async fn test_get_next_nonce_sequential() {
        let manager = create_test_manager().await;
        let n1 = manager.get_next_nonce("0x1234", "eth").await.unwrap();
        let n2 = manager.get_next_nonce("0x1234", "eth").await.unwrap();
        let n3 = manager.get_next_nonce("0x1234", "eth").await.unwrap();
        
        assert_eq!(n1, 0);
        assert_eq!(n2, 1);
        assert_eq!(n3, 2);
    }
    
    #[tokio::test]
    async fn test_get_next_nonce_different_addresses() {
        let manager = create_test_manager().await;
        let n1 = manager.get_next_nonce("0xaaaa", "eth").await.unwrap();
        let n2 = manager.get_next_nonce("0xbbbb", "eth").await.unwrap();
        
        // 不同address的nonce应该独立
        assert_eq!(n1, 0);
        assert_eq!(n2, 0);
    }
    
    #[tokio::test]
    async fn test_get_next_nonce_different_networks() {
        let manager = create_test_manager().await;
        let n1 = manager.get_next_nonce("0x1234", "eth").await.unwrap();
        let n2 = manager.get_next_nonce("0x1234", "bsc").await.unwrap();
        
        // 同一address在不同network的nonce应该独立
        assert_eq!(n1, 0);
        assert_eq!(n2, 0);
    }
    
    #[tokio::test]
    async fn test_mark_nonce_used() {
        let manager = create_test_manager().await;
        let _ = manager.mark_nonce_used("0x5678", "eth", 5).await;
        let next = manager.get_next_nonce("0x5678", "eth").await.unwrap();
        assert!(next > 5);
    }
    
    #[tokio::test]
    async fn test_mark_nonce_used_lower_than_current() {
        let manager = create_test_manager().await;
        let _ = manager.get_next_nonce("0xabcd", "eth").await; // 0
        let _ = manager.get_next_nonce("0xabcd", "eth").await; // 1
        let _ = manager.get_next_nonce("0xabcd", "eth").await; // 2
        
        // 标记一个较小的nonce
        let _ = manager.mark_nonce_used("0xabcd", "eth", 1).await;
        
        // 应该不影响当前nonce
        let next = manager.get_next_nonce("0xabcd", "eth").await.unwrap();
        assert_eq!(next, 3);
    }
    
    #[tokio::test]
    async fn test_reset_nonce() {
        let manager = create_test_manager().await;
        let _ = manager.get_next_nonce("0x9999", "eth").await;
        let _ = manager.get_next_nonce("0x9999", "eth").await;
        
        let _ = manager.reset_nonce("0x9999", "eth").await;
        let nonce = manager.get_next_nonce("0x9999", "eth").await.unwrap();
        assert_eq!(nonce, 0);
    }
    
    #[tokio::test]
    async fn test_reset_nonce_nonexistent_address() {
        let manager = create_test_manager().await;
        let result = manager.reset_nonce("0xnever", "eth").await;
        // 重置不存在的address应该success（不做操作）
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_nonce_concurrent_same_address() {
        let manager = std::sync::Arc::new(create_test_manager().await);
        let address = "0xconcurrent";
        
        let mut handles = vec![];
        for _ in 0..10 {
            let m = manager.clone();
            let a = address.to_string();
            let handle = tokio::spawn(async move {
                m.get_next_nonce(&a, "eth").await.unwrap()
            });
            handles.push(handle);
        }
        
        let mut nonces = vec![];
        for handle in handles {
            nonces.push(handle.await.unwrap());
        }
        
        // 所有nonce应该唯一
        nonces.sort();
        for i in 0..nonces.len() {
            assert_eq!(nonces[i], i as u64);
        }
    }
}

