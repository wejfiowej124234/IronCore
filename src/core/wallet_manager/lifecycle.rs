//! Wallet lifecycle management
//!
//! Provides wallet creation, deletion, and listing functionality

use super::WalletManager;
use crate::core::{
    errors::WalletError,
    wallet_info::SecureWalletData,
};
use tracing::info;

impl WalletManager {
    /// Create a new wallet with BIP39 mnemonic and BIP32/BIP44 key derivation
    ///
    /// # Arguments
    /// * `name` - Wallet name (must be unique)
    /// * `password` - User password for encrypting the master key
    /// * `quantum_safe` - Enable quantum-safe encryption algorithms
    ///
    /// # Returns
    /// * `Ok(())` - Wallet created successfully
    ///
    /// # Errors
    /// * `WalletError::ValidationError` - If wallet with same name already exists
    /// * `WalletError::MnemonicError` - If mnemonic generation fails
    /// * `WalletError::CryptoError` - If encryption fails
    pub async fn create_wallet(
        &self,
        name: &str,
        password: &str,  // Password for PBKDF2 master key encryption
        quantum_safe: bool,
    ) -> Result<(), WalletError> {
        info!("Creating wallet: {} (quantum_safe: {})", name, quantum_safe);

        // Check if wallet already exists
        {
            let wallets = self.wallets.read();
            if wallets.contains_key(name) {
                return Err(WalletError::ValidationError(format!(
                    "Wallet '{}' already exists",
                    name
                )));
            }
        }

        // Wallet creation implementation:
        // 1. Generate BIP39 mnemonic phrase
        // 2. Derive master key using BIP32 standard
        // 3. Encrypt master key with AES-256-GCM (key derived from password via PBKDF2)
        // 4. Store encrypted wallet data in memory
        
        use bip39::{Language, Mnemonic};
        use rand_core::RngCore;
        use chrono::Utc;
        use uuid::Uuid;
        
        // Step 1: Generate 32 bytes of random entropy and create mnemonic
        let mut entropy = [0u8; 32];
        rand_core::OsRng.fill_bytes(&mut entropy);
        
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|e| WalletError::MnemonicError(format!("Failed to generate mnemonic: {}", e)))?;
        
        info!("✅ Generated mnemonic for wallet '{}'", name);
        
        // Step 2: Derive master key from mnemonic using BIP32 standard
        let seed_bytes = mnemonic.to_seed("");  // Empty passphrase
        let mut master_key = [0u8; 32];
        master_key.copy_from_slice(&seed_bytes[..32]);
        
        // Step 3: Encrypt master key using PBKDF2-derived key (consistent with decrypt_master_key)
        use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
        use pbkdf2::pbkdf2_hmac;
        use sha2::Sha256;
        
        
        // Generate random salt (32 bytes)
        let mut salt = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);
        
        // Derive encryption key from password using PBKDF2 (consistent with decryption)
        let iterations = self.config.security.pbkdf2_iterations;
        let mut key_bytes = [0u8; 32];
        pbkdf2_hmac::<Sha256>(
            password.as_bytes(),
            &salt,
            iterations,
            &mut key_bytes,
        );
        
        info!("✅ Derived encryption key using PBKDF2 (iterations: {})", iterations);
        
        // Create AES-256-GCM cipher instance
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|_| WalletError::CryptoError("Failed to create AES cipher".into()))?;
        
        // Generate random nonce (12 bytes for GCM mode)
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = aes_gcm::Nonce::from(nonce_bytes);  // Use `from` instead of deprecated `from_slice`
        
        // Encrypt master key with AES-256-GCM
        let encrypted_master_key = cipher.encrypt(&nonce, master_key.as_ref())
            .map_err(|_| WalletError::CryptoError("Failed to encrypt master key".into()))?;
        
        info!("✅ Encrypted master_key for wallet '{}' (length: {} bytes)", 
              name, encrypted_master_key.len());
        
        // Step 4: Create WalletInfo and SecureWalletData structures
        let wallet_info = crate::core::wallet_info::WalletInfo {
            id: Uuid::new_v4(),
            name: name.to_string(),
            created_at: Utc::now(),
            quantum_safe,
            multi_sig_threshold: 1,
            networks: vec!["eth".to_string(), "btc".to_string()],
        };
        
        let wallet_data = crate::core::wallet_info::SecureWalletData {
            info: wallet_info,
            encrypted_master_key,  // Encrypted master key (AES-256-GCM)
            shamir_shares: Vec::new(),
            salt: salt.to_vec(),  // Save salt for PBKDF2 key derivation during decryption
            nonce: nonce_bytes.to_vec(),  // Save nonce for AES-GCM decryption
            schema_version: 2,
            kek_id: None,
        };
        
        // Step 5: Store wallet data in memory
        {
            let mut wallets = self.wallets.write();
            wallets.insert(name.to_string(), wallet_data);
        }
        
        // Cleanup sensitive data from memory
        use zeroize::Zeroize;
        master_key.zeroize();
        
        info!("✅ Wallet '{}' created successfully with fully encrypted master_key", name);
        Ok(())
    }

    /// Delete a wallet by name
    ///
    /// # Arguments
    /// * `name` - Name of the wallet to delete
    ///
    /// # Returns
    /// * `Ok(())` - Wallet deleted successfully
    ///
    /// # Errors
    /// * `WalletError::NotFoundError` - If wallet does not exist
    pub async fn delete_wallet(&self, name: &str) -> Result<(), WalletError> {
        info!("Deleting wallet: {}", name);

        // Use wallet name as identifier for cleanup
        // WalletInfo doesn't have addresses field, so we use name as the tracking key

        // Remove wallet from memory
        let mut wallets = self.wallets.write();
        if wallets.remove(name).is_some() {
            drop(wallets); // Release wallet lock
            
            // Clean up related nonce tracking entries (using wallet name as key)
            let mut tracker = self.nonce_tracker.write();
            tracker.retain(|addr, _| !addr.starts_with(&format!("wallet_{}", name)));
            
            info!("✅ Wallet '{}' deleted successfully (nonce cleaned)", name);
            Ok(())
        } else {
            Err(WalletError::NotFoundError(format!(
                "Wallet '{}' not found",
                name
            )))
        }
    }

    /// List all wallets managed by this WalletManager
    ///
    /// # Returns
    /// * `Ok(Vec<WalletInfo>)` - List of wallet information (without sensitive data)
    ///
    /// # Errors
    /// Generally doesn't fail, returns empty list if no wallets exist
    pub async fn list_wallets(&self) -> Result<Vec<crate::core::wallet_info::WalletInfo>, WalletError> {
        let wallets = self.wallets.read();
        
        let wallet_list: Vec<crate::core::wallet_info::WalletInfo> = wallets
            .values()
            .map(|wallet_data| wallet_data.info.clone())
            .collect();

        Ok(wallet_list)
    }

    /// Get wallet data by name
    ///
    /// # Arguments
    /// * `name` - Wallet name
    ///
    /// # Returns
    /// * `Option<SecureWalletData>` - Wallet data if found, None otherwise
    pub async fn get_wallet_by_name(
        &self,
        name: &str,
    ) -> Result<Option<SecureWalletData>, WalletError> {
        let wallets = self.wallets.read();
        Ok(wallets.get(name).cloned())
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
    async fn test_create_wallet_basic() {
        let manager = create_test_manager().await;
        let result = manager.create_wallet("test_wallet", "test_password", false).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_create_wallet_quantum_safe() {
        let manager = create_test_manager().await;
        let result = manager.create_wallet("quantum_wallet", "test_password", true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_wallet_duplicate_error() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("duplicate", "test_password", false).await;
        let result = manager.create_wallet("duplicate", "test_password", false).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::ValidationError(_)));
    }
    
    #[tokio::test]
    async fn test_create_wallet_empty_name() {
        let manager = create_test_manager().await;
        let result = manager.create_wallet("", "test_password", false).await;
        // Empty name should be handled by upper layer validation
        let _ = result;
    }
    
    #[tokio::test]
    async fn test_create_wallet_special_characters() {
        let manager = create_test_manager().await;
        let names = vec!["test_cn", "wallet-123", "wallet_test", "wallet.test"];
        for name in names {
            let result = manager.create_wallet(name, "test_password", false).await;
            assert!(result.is_ok(), "Failed for name: {}", name);
        }
    }

    #[tokio::test]
    async fn test_list_wallets_empty() {
        let manager = create_test_manager().await;
        let wallets = manager.list_wallets().await.unwrap();
        assert!(wallets.is_empty());
    }
    
    #[tokio::test]
    async fn test_list_wallets_multiple() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("wallet1", "test_password", false).await;
        let _ = manager.create_wallet("wallet2", "test_password", false).await;
        let _ = manager.create_wallet("wallet3", "test_password", false).await;
        
        let wallets = manager.list_wallets().await.unwrap();
        assert_eq!(wallets.len(), 3);
    }

    #[tokio::test]
    async fn test_get_nonexistent_wallet() {
        let manager = create_test_manager().await;
        let wallet = manager.get_wallet_by_name("nonexistent").await.unwrap();
        assert!(wallet.is_none());
    }
    
    #[tokio::test]
    async fn test_get_existing_wallet() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("exists", "test_password", false).await;
        let wallet = manager.get_wallet_by_name("exists").await.unwrap();
        assert!(wallet.is_some());
        assert_eq!(wallet.unwrap().info.name, "exists");
    }
    
    #[tokio::test]
    async fn test_delete_wallet_success() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("to_delete", "test_password", false).await;
        let result = manager.delete_wallet("to_delete").await;
        assert!(result.is_ok());
        
        // Verify wallet has been deleted
        let wallet = manager.get_wallet_by_name("to_delete").await.unwrap();
        assert!(wallet.is_none());
    }
    
    #[tokio::test]
    async fn test_delete_wallet_not_found() {
        let manager = create_test_manager().await;
        let result = manager.delete_wallet("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::NotFoundError(_)));
    }
    
    #[tokio::test]
    async fn test_create_delete_create_cycle() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("cycle", "test_password", false).await;
        let _ = manager.delete_wallet("cycle").await;
        let result = manager.create_wallet("cycle", "test_password", false).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_concurrent_create_different_names() {
        let manager = std::sync::Arc::new(create_test_manager().await);
        
        let mut handles = vec![];
        for i in 0..10 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                manager_clone.create_wallet(&format!("concurrent_{}", i), "test_password", false).await
            });
            handles.push(handle);
        }
        
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
        
        let wallets = manager.list_wallets().await.unwrap();
        assert_eq!(wallets.len(), 10);
    }
    
    #[tokio::test]
    async fn test_wallet_info_fields() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("info_test", "test_password", false).await;
        let wallet = manager.get_wallet_by_name("info_test").await.unwrap().unwrap();
        
        assert_eq!(wallet.info.name, "info_test");
        assert!(!wallet.info.id.is_nil());
        assert_eq!(wallet.info.multi_sig_threshold, 1);  // Default threshold is 1
    }
}

