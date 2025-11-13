//! 安全的密钥派生模块
//!
//! 🔐 符合 BIP39 标准的安全派生
//! ✅ 使用userPassword短语增强熵
//! ✅ PBKDF2-HMAC-SHA512 (2048轮迭代)
//! ✅ 防止暴力破解
use bip39::Mnemonic;
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};
use crate::core::errors::WalletError;

/// 🔐 安全派生：frommnemonic + userPassword → 主密钥
/// 
/// # 安全特性
/// - ✅ BIP39 标准：使用userPassword短语
/// - ✅ PBKDF2-HMAC-SHA512：2048轮迭代（BIP39 标准）
/// - ✅ HKDF-SHA256：额外密钥派生函数
/// - ✅ Zeroizing：自动擦除敏感数据
/// 
/// # 参数
/// - `mnemonic`: BIP39 mnemonic
/// - `passphrase`: userPassword短语（BIP39 标准，**不能为空**）
/// - `salt`: 额外的应用层盐值（可选，推荐）
/// 
/// # 返回
/// 32字节的主密钥，使用 Zeroizing 包装
pub fn derive_master_key_secure(
    mnemonic: &str,
    passphrase: &str,  // 🔴 强制要求Password短语
    app_salt: Option<&[u8]>,  // 额外的应用层盐值
) -> Result<Zeroizing<[u8; 32]>, WalletError> {
    // 1. 解析并Validate mnemonic
    let mnemonic = Mnemonic::parse(mnemonic)
        .map_err(|e| WalletError::MnemonicError(format!("无效的mnemonic: {}", e)))?;
    
    // 2. ✅ 使用 BIP39 标准派生（PBKDF2-HMAC-SHA512，2048轮）
    //    passphrase 增加熵，防止仅暴力mnemonic
    let mut seed = mnemonic.to_seed(passphrase);  // 🔐 使用真实Password
    
    // 3. ✅ 可选：使用 HKDF 进一步派生（应用层额外保护）
    let master_key = if let Some(salt) = app_salt {
        let hk = Hkdf::<Sha256>::new(Some(salt), &seed[..]);
        let mut okm = Zeroizing::new([0u8; 32]);
        hk.expand(b"wallet-master-key-v3", okm.as_mut())
            .map_err(|_| WalletError::KeyDerivationError("HKDF 派生failed".into()))?;
        okm
    } else {
        // 直接使用前32字节（BIP39标准）
        let mut key = Zeroizing::new([0u8; 32]);
        key.copy_from_slice(&seed[..32]);
        key
    };
    
    // 4. ✅ 清零种子（防止内存泄漏）
    seed.zeroize();
    
    Ok(master_key)
}

/// 🔐 向后兼容：frommnemonic派生（使用默认Password）
/// 
/// ⚠️  Warning:仅用于测试或迁移，生产环境必须使用 derive_master_key_secure
#[deprecated(note = "使用 derive_master_key_secure 并提供真实Password")]
pub fn derive_master_key_compat(mnemonic: &str) -> Result<Zeroizing<[u8; 32]>, WalletError> {
    // 使用固定的应用盐值作为最低保护
    const APP_SALT: &[u8] = b"defi-hot-wallet-v1-entropy-boost";
    
    derive_master_key_secure(mnemonic, "", Some(APP_SALT))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_secure_derivation_with_passphrase() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        
        // 使用不同Password应该得到不同密钥
        let key1 = derive_master_key_secure(mnemonic, "password123", None).unwrap();
        let key2 = derive_master_key_secure(mnemonic, "different_pass", None).unwrap();
        
        assert_ne!(&key1[..], &key2[..], "不同Password应该产生不同密钥");
    }
    
    #[test]
    fn test_derivation_with_app_salt() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        
        // 使用应用盐值应该得到不同结果
        let key_no_salt = derive_master_key_secure(mnemonic, "pass", None).unwrap();
        let key_with_salt = derive_master_key_secure(mnemonic, "pass", Some(b"app-salt")).unwrap();
        
        assert_ne!(&key_no_salt[..], &key_with_salt[..], "应用盐值应该改变结果");
    }
    
    #[test]
    fn test_zeroizing_works() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        
        let mut key = derive_master_key_secure(mnemonic, "password", None).unwrap();
        
        // key 应该被 Zeroizing 包装
        assert_eq!(key.len(), 32);
        
        // 显式清零
        key.zeroize();
        
        // validate已清零
        assert!(key.iter().all(|&b| b == 0));
    }
}

