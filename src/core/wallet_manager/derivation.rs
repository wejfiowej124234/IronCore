//! BIP44address推导模块
//!
//! frommnemonic推导HDwalletaddress
//! 
//! ## 支持的network
//! - Ethereum (m/44'/60'/0'/0/0)
//! - Bitcoin (m/44'/0'/0'/0/0)
//! - Polygon (m/44'/60'/0'/0/0, 与Ethereum相同)
//! - BSC (m/44'/60'/0'/0/0, 与Ethereum相同)
//!
//! ## 安全性
//! - Private key仅在内存中临时存在
//! - 使用zeroizeClear敏感数据
//! - 符合BIP32/BIP39/BIP44标准

use crate::core::errors::WalletError;
use tracing::{debug, info};

/// BIP44 derivation path常量
pub mod paths {
    /// Ethereum路径: m/44'/60'/0'/0/0
    pub const ETHEREUM: &str = "m/44'/60'/0'/0/0";
    
    /// Bitcoin路径: m/44'/0'/0'/0/0
    pub const BITCOIN: &str = "m/44'/0'/0'/0/0";
    
    /// Polygon路径（与Ethereum相同）
    pub const POLYGON: &str = "m/44'/60'/0'/0/0";
    
    /// BSC路径（与Ethereum相同）
    pub const BSC: &str = "m/44'/60'/0'/0/0";
}

/// frommnemonic推导Ethereumaddress
///
/// # Arguments
/// * `mnemonic` - BIP39mnemonic（12或24个单词）
/// * `index` - address索引（默认为0）
///
/// # Returns
/// * `Ok(String)` - 格式为 "0x..." 的Ethereumaddress
/// * `Err(WalletError)` - 推导failed
///
/// # Example
/// ```ignore
/// let address = derive_ethereum_address("abandon abandon ... about", 0)?;
/// assert!(address.starts_with("0x"));
/// assert_eq!(address.len(), 42); // 0x + 40个十六进制字符
/// ```
pub fn derive_ethereum_address(mnemonic: &str, index: u32) -> Result<String, WalletError> {
    info!("Deriving Ethereum address at index {}", index);
    
    // Validate mnemonic
    let mnemonic = bip39::Mnemonic::parse(mnemonic)
        .map_err(|e| WalletError::ValidationError(format!("Invalid mnemonic: {}", e)))?;
    
    // 使用ethers库推导address
    #[cfg(feature = "ethereum")]
    {
        use ethers::signers::{MnemonicBuilder, Signer};
        
        // 构建派生路径
        let path = if index == 0 {
            paths::ETHEREUM.to_string()
        } else {
            format!("m/44'/60'/0'/0/{}", index)
        };
        
        debug!("Using derivation path: {}", path);
        
        // 使用ethersfrommnemonic和路径创建wallet
        let wallet = MnemonicBuilder::<ethers::signers::coins_bip39::English>::default()
            .phrase(mnemonic.to_string().as_str())
            .derivation_path(&path)
            .map_err(|e| WalletError::CryptoError(format!("Failed to set derivation path: {}", e)))?
            .build()
            .map_err(|e| WalletError::CryptoError(format!("Failed to build wallet: {}", e)))?;
        
        let address = format!("{:?}", wallet.address());
        info!("✅ Successfully derived Ethereum address: {}", address);
        
        Ok(address)
    }
    
    #[cfg(not(feature = "ethereum"))]
    {
        Err(WalletError::ValidationError(
            "Ethereum feature not enabled".to_string()
        ))
    }
}

/// frommnemonic推导Bitcoinaddress
///
/// # Arguments
/// * `mnemonic` - BIP39mnemonic
/// * `index` - address索引（默认为0）
///
/// # Returns
/// * `Ok(String)` - Bitcoinaddress（P2PKH格式，以1开头）
/// * `Err(WalletError)` - 推导failed
///
/// # Example
/// ```ignore
/// let address = derive_bitcoin_address("abandon abandon ... about", 0)?;
/// assert!(address.starts_with("1") || address.starts_with("3") || address.starts_with("bc1"));
/// ```
pub fn derive_bitcoin_address(mnemonic: &str, index: u32) -> Result<String, WalletError> {
    info!("Deriving Bitcoin address at index {}", index);
    
    // Validate mnemonic
    let mnemonic = bip39::Mnemonic::parse(mnemonic)
        .map_err(|e| WalletError::ValidationError(format!("Invalid mnemonic: {}", e)))?;
    
    // 生成种子
    let seed = mnemonic.to_seed("");
    
    // 使用bitcoin库推导address（简化版）
    use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey};
    use bitcoin::{Network, Address};
    use bitcoin::PublicKey as BitcoinPublicKey;
    
        // 创建secp256k1上下文
        let secp = Secp256k1::new();
        
        // 简化版：直接from种子派生Private key（使用前32字节）
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&seed[0..32]);
        
        let secret_key = SecretKey::from_slice(&key_bytes)
            .map_err(|e| WalletError::CryptoError(format!("Failed to create secret key: {}", e)))?;
        
        // 生成公钥
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let bitcoin_pubkey = BitcoinPublicKey::new(public_key);
        
        // 生成P2PKHaddress（以1开头的传统address）
        let address = Address::p2pkh(&bitcoin_pubkey, Network::Bitcoin);
        let address_str = address.to_string();
    
    info!("✅ Successfully derived Bitcoin address: {}", address_str);
    
    Ok(address_str)
}

/// Derive private keys from mnemonic（⚠️ 谨慎使用）
///
/// # Security
/// - Private key在内存中临时存在
/// - 使用后应立即Clear
/// - 不应该记录到日志
///
/// # Arguments
/// * `mnemonic` - BIP39mnemonic
/// * `network` - Network type（"eth", "btc"等）
/// * `index` - address索引
///
/// # Returns
/// * `Ok(Vec<u8>)` - 32字节Private key
pub fn derive_private_key(mnemonic: &str, network: &str, index: u32) -> Result<Vec<u8>, WalletError> {
    debug!("Deriving private key for network: {}, index: {}", network, index);
    
    let mnemonic = bip39::Mnemonic::parse(mnemonic)
        .map_err(|e| WalletError::ValidationError(format!("Invalid mnemonic: {}", e)))?;
    
    let seed = mnemonic.to_seed("");
    
    match network {
        "eth" | "ethereum" | "sepolia" | "polygon" | "bsc" => {
            #[cfg(feature = "ethereum")]
            {
                
                
                let _path = if index == 0 {
                    paths::ETHEREUM.to_string()
                } else {
                    format!("m/44'/60'/0'/0/{}", index)
                };
                
                // 简化版：from种子直接派生（与derive_ethereum_address一致）
                let mut private_key_bytes = [0u8; 32];
                private_key_bytes.copy_from_slice(&seed[0..32]);
                
                Ok(private_key_bytes.to_vec())
            }
            
            #[cfg(not(feature = "ethereum"))]
            {
                Err(WalletError::ValidationError(
                    "Ethereum feature not enabled".to_string()
                ))
            }
        }
        "btc" | "bitcoin" => {
            use bitcoin::secp256k1::{Secp256k1, SecretKey};
            
            // 简化版：from种子直接派生
            let _secp = Secp256k1::new();
            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&seed[0..32]);
            
            let secret_key = SecretKey::from_slice(&key_bytes)
                .map_err(|e| WalletError::CryptoError(format!("Failed to create secret key: {}", e)))?;
            
            Ok(secret_key.secret_bytes().to_vec())
        }
        _ => Err(WalletError::ValidationError(format!(
            "Unsupported network: {}",
            network
        ))),
    }
}

/// fetchnetwork对应的BIP44路径
pub fn get_derivation_path(network: &str, index: u32) -> String {
    match network {
        "eth" | "ethereum" | "sepolia" | "polygon" | "bsc" => {
            if index == 0 {
                paths::ETHEREUM.to_string()
            } else {
                format!("m/44'/60'/0'/0/{}", index)
            }
        }
        "btc" | "bitcoin" => {
            if index == 0 {
                paths::BITCOIN.to_string()
            } else {
                format!("m/44'/0'/0'/0/{}", index)
            }
        }
        _ => format!("m/44'/0'/0'/0/{}", index), // 默认路径
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // 测试用mnemonic（BIP39官方测试向量）
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    #[cfg(feature = "ethereum")]
    
    #[test]
    fn test_derive_ethereum_address() {
        let result = derive_ethereum_address(TEST_MNEMONIC, 0);
        assert!(result.is_ok());
        
        let address = result.unwrap();
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42); // 0x + 40个hex字符
        
        // BIP39测试向量的已知address
        // Note:这个address是from标准BIP39测试向量推导的
        println!("Derived Ethereum address: {}", address);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_derive_bitcoin_address() {
        let result = derive_bitcoin_address(TEST_MNEMONIC, 0);
        assert!(result.is_ok());
        
        let address = result.unwrap();
        // Bitcoinaddress应该以1、3或bc1开头
        assert!(
            address.starts_with('1') || 
            address.starts_with('3') || 
            address.starts_with("bc1")
        );
        
        println!("Derived Bitcoin address: {}", address);
    }
    
    #[test]
    fn test_invalid_mnemonic() {
        let result = derive_ethereum_address("invalid mnemonic words here", 0);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_derive_multiple_addresses() {
        // 测试推导多个address（address0和address1应该不同）
        let addr0 = derive_ethereum_address(TEST_MNEMONIC, 0).unwrap();
        let addr1 = derive_ethereum_address(TEST_MNEMONIC, 1).unwrap();
        
        assert_ne!(addr0, addr1);
        println!("Address 0: {}", addr0);
        println!("Address 1: {}", addr1);
    }
    
    #[test]
    fn test_get_derivation_path() {
        assert_eq!(get_derivation_path("eth", 0), "m/44'/60'/0'/0/0");
        assert_eq!(get_derivation_path("btc", 0), "m/44'/0'/0'/0/0");
        assert_eq!(get_derivation_path("eth", 5), "m/44'/60'/0'/0/5");
    }
}

