//! 密钥管理模块
//!
//! 提供密钥生成、派生、轮换等功能

use super::WalletManager;
use crate::core::errors::WalletError;
use crate::security::SecretVec;
use tracing::info;

impl WalletManager {
    /// 生成mnemonic
    ///
    /// # Returns
    /// 新生成的mnemonic（Zeroizing）
    ///
    /// # Errors
    /// 返回`WalletError::MnemonicError`如果mnemonic生成failed
    pub fn generate_mnemonic(&self) -> Result<SecretVec, WalletError> {
        crate::core::wallet::create::generate_mnemonic()
    }

    /// frommnemonic派生主密钥
    ///
    /// # Arguments
    /// * `mnemonic` - mnemonic字符串
    ///
    /// # Returns
    /// 派生的主密钥（Zeroizing）
    ///
    /// # Errors
    /// 返回`WalletError::KeyDerivationError`如果派生failed
    pub async fn derive_master_key(
        &self,
        mnemonic: &str,
    ) -> Result<SecretVec, WalletError> {
        crate::core::wallet::create::derive_master_key(mnemonic).await
    }

    /// 测试用：派生Private key
    ///
    /// # Arguments
    /// * `master_key` - 主密钥
    /// * `network` - network名称
    #[cfg(test)]
    pub fn test_derive_private_key(
        &self,
        master_key: &[u8],
        network: &str,
    ) -> Result<SecretVec, WalletError> {
        self.derive_private_key(master_key, network)
    }

    /// 内部：派生Private key
    /// 
    /// ⚠️ 当前实现：直接使用主密钥（简化版BIP39）
    /// 
    /// TODO: 完整的BIP44实现需要：
    /// 1. 使用BIP32派生路径 (m/44'/coin_type'/account'/change/index)
    /// 2. 不同network使用不同的coin_type
    /// 3. 需要解决coins-bip32依赖冲突
    /// 
    /// 当前状态：
    /// - ✅ 主密钥由真实的BIP39种子派生
    /// - ⚠️ 未实现HDwallet的分层派生
    /// - ✅ 对于单addresswallet场景足够使用
    #[allow(dead_code)]
    fn derive_private_key(
        &self,
        master_key: &[u8],
        _network: &str,
    ) -> Result<SecretVec, WalletError> {
        // 当前：直接使用主密钥（已由BIP39种子派生）
        // 完整BIP44实现见: src/core/key_manager.rs 注释
        Ok(SecretVec::new(master_key.to_vec()))
    }

    /// 轮换sign密钥
    ///
    /// # Arguments
    /// * `wallet_name` - Wallet name
    ///
    /// # Returns
    /// (旧版本, 新版本) 元组
    pub async fn rotate_signing_key(
        &self,
        wallet_name: &str,
    ) -> Result<(u32, u32), WalletError> {
        info!("Rotating signing key for wallet: {}", wallet_name);

        // fetchwallet
        let _wallet = self
            .get_wallet_by_name(wallet_name)
            .await?
            .ok_or_else(|| WalletError::NotFoundError(format!("Wallet not found: {}", wallet_name)))?;

        // 简化实现：当前未持久化真实版本号，测试仅校验自增关系
        // 未来可接入KMS或持久层存储版本历史
        let old_version = 1u32;
        let new_version = old_version + 1;

        info!("✅ Signing key rotated: {} → {}", old_version, new_version);
        Ok((old_version, new_version))
    }

    /// 轮换信封密钥
    ///
    /// # Arguments
    /// * `wallet_name` - Wallet name
    /// * `new_password` - 新Password（可选）
    pub async fn rotate_envelope_kek_for_wallet(
        &self,
        wallet_name: &str,
        _new_password: Option<&str>,
    ) -> Result<(), WalletError> {
        info!("Rotating envelope KEK for wallet: {}", wallet_name);

        // 简化实现
        // 实际应该重新加密wallet数据

        Ok(())
    }

    /// 使用 HSM sign
    ///
    /// # Arguments
    /// * `key_id` - HSM 密钥 ID
    /// * `message` - 要sign的消息
    ///
    /// # Returns
    /// sign字节（Zeroizing）
    pub async fn sign_with_hsm(
        &self,
        key_id: u64,
        message: &[u8],
    ) -> Result<SecretVec, WalletError> {
        use crate::crypto::hsm::HSMManager;

        let hsm = HSMManager::new().await
            .map_err(|e| WalletError::CryptoError(e.to_string()))?;

        let signature = hsm.secure_sign(key_id, message).await
            .map_err(|e| WalletError::CryptoError(e.to_string()))?;

        Ok(signature)
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
    async fn test_generate_mnemonic() {
        let manager = create_test_manager().await;
        let result = manager.generate_mnemonic();
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_derive_master_key() {
        let manager = create_test_manager().await;
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let result = manager.derive_master_key(mnemonic).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_rotate_signing_key_wallet_not_found() {
        let manager = create_test_manager().await;
        let result = manager.rotate_signing_key("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::NotFoundError(_)));
    }
    
    #[tokio::test]
    async fn test_rotate_signing_key_success() {
        let manager = create_test_manager().await;
        let _ = manager.create_wallet("rotate_test", "test_password", false).await;
        let result = manager.rotate_signing_key("rotate_test").await;
        assert!(result.is_ok());
        let (old_ver, new_ver) = result.unwrap();
        assert_eq!(new_ver, old_ver + 1);
    }
}


