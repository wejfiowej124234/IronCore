//! Bitcoin address生成和validate
//! 
//! 支持三种address类型：
//! - Legacy (P2PKH): 1xxx
//! - SegWit (P2WPKH): bc1qxxx
//! - Taproot (P2TR): bc1pxxx

use crate::core::errors::WalletError;
use bitcoin::address::Address;
use bitcoin::key::{TapTweak, UntweakedPublicKey};
use bitcoin::secp256k1::{PublicKey as Secp256k1PublicKey, Secp256k1, XOnlyPublicKey};
use bitcoin::{Network, PublicKey as BitcoinPublicKey};
use std::str::FromStr;
use tracing::{debug, info};

/// Bitcoin address类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    /// Legacy (P2PKH): Pay to Public Key Hash
    /// address格式: 1xxx
    Legacy,
    
    /// SegWit (P2WPKH): Pay to Witness Public Key Hash
    /// address格式: bc1qxxx (主网) 或 tb1qxxx (测试网)
    SegWit,
    
    /// Taproot (P2TR): Pay to Taproot
    /// address格式: bc1pxxx (主网) 或 tb1pxxx (测试网)
    Taproot,
}

/// Bitcoin address包装器
pub struct BitcoinAddress;

impl BitcoinAddress {
    /// from公钥生成 Legacy address (P2PKH)
    pub fn from_public_key_legacy(
        public_key: &Secp256k1PublicKey,
        network: Network,
    ) -> Result<String, WalletError> {
        info!("生成 Legacy address (P2PKH)");
        
        // 转换 secp256k1::PublicKey 到 bitcoin::PublicKey
        let btc_pubkey = BitcoinPublicKey::new(*public_key);
        let address = Address::p2pkh(&btc_pubkey, network);
        
        debug!("✅ Legacy address生成success: {}", address);
        Ok(address.to_string())
    }
    
    /// from公钥生成 SegWit address (P2WPKH)
    pub fn from_public_key_segwit(
        public_key: &Secp256k1PublicKey,
        network: Network,
    ) -> Result<String, WalletError> {
        info!("生成 SegWit address (P2WPKH)");
        
        // 转换 secp256k1::PublicKey 到 bitcoin::PublicKey
        let btc_pubkey = BitcoinPublicKey::new(*public_key);
        let address = Address::p2wpkh(&btc_pubkey, network)
            .map_err(|e| WalletError::AddressGenerationFailed(format!("SegWit address生成failed: {}", e)))?;
        
        debug!("✅ SegWit address生成success: {}", address);
        Ok(address.to_string())
    }
    
    /// from公钥生成 Taproot address (P2TR)
    pub fn from_public_key_taproot(
        public_key: &Secp256k1PublicKey,
        network: Network,
    ) -> Result<String, WalletError> {
        info!("生成 Taproot address (P2TR)");
        
        let secp = Secp256k1::new();
        
        // 将 secp256k1 公钥转换为 x-only 公钥
        let xonly_pubkey = XOnlyPublicKey::from(*public_key);
        
        // 创建未调整的公钥
        let untweaked_pubkey = UntweakedPublicKey::from(xonly_pubkey);
        
        // 应用 Taproot 调整
        let (tweaked_pubkey, _parity) = untweaked_pubkey.tap_tweak(&secp, None);
        
        // 创建 P2TR address
        let address = Address::p2tr(&secp, tweaked_pubkey.into(), None, network);
        
        debug!("✅ Taproot address生成success: {}", address);
        Ok(address.to_string())
    }
    
    /// 根据address类型from公钥生成address
    pub fn from_public_key(
        public_key: &Secp256k1PublicKey,
        address_type: AddressType,
        network: Network,
    ) -> Result<String, WalletError> {
        match address_type {
            AddressType::Legacy => Self::from_public_key_legacy(public_key, network),
            AddressType::SegWit => Self::from_public_key_segwit(public_key, network),
            AddressType::Taproot => Self::from_public_key_taproot(public_key, network),
        }
    }
    
    /// validate Bitcoin address格式
    pub fn validate(address_str: &str, network: Network) -> Result<bool, WalletError> {
        match Address::from_str(address_str) {
            Ok(address) => {
                // checknetwork是否匹配
                let is_valid = address.is_valid_for_network(network);
                Ok(is_valid)
            }
            Err(_) => Ok(false),
        }
    }
    
    /// 检测address类型
    pub fn detect_type(address_str: &str) -> Result<AddressType, WalletError> {
        let address = Address::from_str(address_str)
            .map_err(|e| WalletError::InvalidAddress(format!("无效的address: {}", e)))?;
        
        // 根据address的 payload 判断类型
        use bitcoin::address::Payload;
        
        match address.payload() {
            Payload::PubkeyHash(_) => Ok(AddressType::Legacy),
            Payload::WitnessProgram(ref program) => {
                if program.version().to_num() == 0 {
                    Ok(AddressType::SegWit)
                } else if program.version().to_num() == 1 {
                    Ok(AddressType::Taproot)
                } else {
                    Err(WalletError::InvalidAddress("未知的 Witness 版本".to_string()))
                }
            }
            _ => Err(WalletError::InvalidAddress("不支持的address类型".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::SecretKey;
    use rand::RngCore;
    
    fn generate_test_keypair() -> (SecretKey, Secp256k1PublicKey) {
        let secp = Secp256k1::new();
        use rand::rngs::OsRng;  // 使用Password学安全的RNG
        let mut rng = OsRng;  // ✅ 即使在测试中也使用安全RNG
        let mut secret_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_bytes);
        
        let secret_key = SecretKey::from_slice(&secret_bytes).unwrap();
        let public_key = Secp256k1PublicKey::from_secret_key(&secp, &secret_key);
        
        (secret_key, public_key)
    }
    
    #[test]
    fn test_legacy_address_generation() {
        let (_secret, public_key) = generate_test_keypair();
        let address = BitcoinAddress::from_public_key_legacy(&public_key, Network::Testnet).unwrap();
        
        assert!(address.starts_with('m') || address.starts_with('n')); // 测试网 Legacy address
        assert!(BitcoinAddress::validate(&address, Network::Testnet).unwrap());
    }
    
    #[test]
    fn test_segwit_address_generation() {
        let (_secret, public_key) = generate_test_keypair();
        let address = BitcoinAddress::from_public_key_segwit(&public_key, Network::Testnet).unwrap();
        
        assert!(address.starts_with("tb1q")); // 测试网 SegWit address
        assert!(BitcoinAddress::validate(&address, Network::Testnet).unwrap());
    }
    
    #[test]
    fn test_taproot_address_generation() {
        let (_secret, public_key) = generate_test_keypair();
        let address = BitcoinAddress::from_public_key_taproot(&public_key, Network::Testnet).unwrap();
        
        assert!(address.starts_with("tb1p")); // 测试网 Taproot address
        assert!(BitcoinAddress::validate(&address, Network::Testnet).unwrap());
    }
    
    #[test]
    fn test_mainnet_legacy_address() {
        let (_secret, public_key) = generate_test_keypair();
        let address = BitcoinAddress::from_public_key_legacy(&public_key, Network::Bitcoin).unwrap();
        
        assert!(address.starts_with('1')); // 主网 Legacy address
        assert!(BitcoinAddress::validate(&address, Network::Bitcoin).unwrap());
    }
    
    #[test]
    fn test_mainnet_segwit_address() {
        let (_secret, public_key) = generate_test_keypair();
        let address = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
        
        assert!(address.starts_with("bc1q")); // 主网 SegWit address
        assert!(BitcoinAddress::validate(&address, Network::Bitcoin).unwrap());
    }
    
    #[test]
    fn test_mainnet_taproot_address() {
        let (_secret, public_key) = generate_test_keypair();
        let address = BitcoinAddress::from_public_key_taproot(&public_key, Network::Bitcoin).unwrap();
        
        assert!(address.starts_with("bc1p")); // 主网 Taproot address
        assert!(BitcoinAddress::validate(&address, Network::Bitcoin).unwrap());
    }
    
    #[test]
    fn test_address_type_detection() {
        let (_secret, public_key) = generate_test_keypair();
        
        // Legacy
        let legacy_addr = BitcoinAddress::from_public_key_legacy(&public_key, Network::Testnet).unwrap();
        assert_eq!(BitcoinAddress::detect_type(&legacy_addr).unwrap(), AddressType::Legacy);
        
        // SegWit
        let segwit_addr = BitcoinAddress::from_public_key_segwit(&public_key, Network::Testnet).unwrap();
        assert_eq!(BitcoinAddress::detect_type(&segwit_addr).unwrap(), AddressType::SegWit);
        
        // Taproot
        let taproot_addr = BitcoinAddress::from_public_key_taproot(&public_key, Network::Testnet).unwrap();
        assert_eq!(BitcoinAddress::detect_type(&taproot_addr).unwrap(), AddressType::Taproot);
    }
    
    #[test]
    fn test_invalid_address_validation() {
        assert!(!BitcoinAddress::validate("invalid_address", Network::Bitcoin).unwrap());
    }
    
    #[test]
    fn test_network_mismatch() {
        let (_secret, public_key) = generate_test_keypair();
        let testnet_addr = BitcoinAddress::from_public_key_segwit(&public_key, Network::Testnet).unwrap();
        
        // 测试网address在主网上应该无效
        assert!(!BitcoinAddress::validate(&testnet_addr, Network::Bitcoin).unwrap());
    }
    
    // ============ 新增的边界和error测试 ============
    
    #[test]
    fn test_empty_address_validation() {
        // 空address应该无效
        assert!(!BitcoinAddress::validate("", Network::Bitcoin).unwrap());
    }
    
    #[test]
    fn test_malformed_address() {
        // 格式error的address
        let invalid_addresses = vec![
            "1",  // 太短
            "!!!invalid!!!",  // 无效字符
            "bc1qqqqqqqq",  // 长度不对
            "tb1p" ,  // 不完整
            "ABCDEFGHIJK123456",  // 随机字符串
        ];
        
        for addr in invalid_addresses {
            assert!(!BitcoinAddress::validate(addr, Network::Bitcoin).unwrap(), 
                    "address '{}' 应该无效", addr);
        }
    }
    
    #[test]
    fn test_address_case_sensitivity() {
        let (_secret, public_key) = generate_test_keypair();
        let address = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
        
        // Bech32 address应该是小写
        assert_eq!(address, address.to_lowercase());
    }
    
    #[test]
    fn test_all_address_types_testnet() {
        let (_secret, public_key) = generate_test_keypair();
        
        // Legacy Testnet (m 或 n 开头)
        let legacy = BitcoinAddress::from_public_key_legacy(&public_key, Network::Testnet).unwrap();
        assert!(legacy.starts_with('m') || legacy.starts_with('n'));
        
        // SegWit Testnet (tb1q 开头)
        let segwit = BitcoinAddress::from_public_key_segwit(&public_key, Network::Testnet).unwrap();
        assert!(segwit.starts_with("tb1q"));
        
        // Taproot Testnet (tb1p 开头)
        let taproot = BitcoinAddress::from_public_key_taproot(&public_key, Network::Testnet).unwrap();
        assert!(taproot.starts_with("tb1p"));
    }
    
    #[test]
    fn test_address_length_ranges() {
        let (_secret, public_key) = generate_test_keypair();
        
        // Legacy address通常 26-35 字符
        let legacy = BitcoinAddress::from_public_key_legacy(&public_key, Network::Bitcoin).unwrap();
        assert!(legacy.len() >= 26 && legacy.len() <= 35, "Legacy address长度: {}", legacy.len());
        
        // SegWit address通常 42-62 字符
        let segwit = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
        assert!(segwit.len() >= 42 && segwit.len() <= 62, "SegWit address长度: {}", segwit.len());
        
        // Taproot address通常 62 字符
        let taproot = BitcoinAddress::from_public_key_taproot(&public_key, Network::Bitcoin).unwrap();
        assert!(taproot.len() == 62, "Taproot address长度: {}", taproot.len());
    }
    
    #[test]
    fn test_address_uniqueness() {
        // 不同的公钥应该生成不同的address
        let (_, pk1) = generate_test_keypair();
        let (_, pk2) = generate_test_keypair();
        
        let addr1 = BitcoinAddress::from_public_key_segwit(&pk1, Network::Bitcoin).unwrap();
        let addr2 = BitcoinAddress::from_public_key_segwit(&pk2, Network::Bitcoin).unwrap();
        
        assert_ne!(addr1, addr2, "不同公钥应该生成不同address");
    }
    
    #[test]
    fn test_address_deterministic() {
        // 相同的公钥应该总是生成相同的address
        let (secret, _) = generate_test_keypair();
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(
            &bitcoin::secp256k1::Secp256k1::new(),
            &secret
        );
        
        let addr1 = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
        let addr2 = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
        
        assert_eq!(addr1, addr2, "相同公钥应该生成相同address");
    }
    
    #[test]
    fn test_validate_real_bitcoin_addresses() {
        // 真实的比特币address应该validate通过
        let real_addresses = vec![
            ("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa", Network::Bitcoin),  // Genesis block
            ("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", Network::Bitcoin),  // SegWit example
        ];
        
        for (addr, network) in real_addresses {
            assert!(BitcoinAddress::validate(addr, network).unwrap(), 
                    "真实address {} 应该有效", addr);
        }
    }
    
    #[test]
    fn test_cross_network_address_rejection() {
        // 主网address在测试网应该无效
        let mainnet_addresses = vec![
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",  // Legacy mainnet
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",  // SegWit mainnet
        ];
        
        for addr in mainnet_addresses {
            assert!(!BitcoinAddress::validate(addr, Network::Testnet).unwrap(),
                    "主网address {} 在测试网应该无效", addr);
        }
    }
    
    #[test]
    fn test_dispatcher_from_public_key() {
        // 测试通用的 from_public_key 调度器
        let (_secret, public_key) = generate_test_keypair();
        
        let legacy = BitcoinAddress::from_public_key(&public_key, AddressType::Legacy, Network::Bitcoin).unwrap();
        assert!(legacy.starts_with('1') || legacy.starts_with('3'));
        
        let segwit = BitcoinAddress::from_public_key(&public_key, AddressType::SegWit, Network::Bitcoin).unwrap();
        assert!(segwit.starts_with("bc1q"));
        
        let taproot = BitcoinAddress::from_public_key(&public_key, AddressType::Taproot, Network::Bitcoin).unwrap();
        assert!(taproot.starts_with("bc1p"));
    }
    
    #[test]
    fn test_detect_type_edge_cases() {
        // 测试类型检测的边界情况
        assert!(BitcoinAddress::detect_type("").is_err());
        assert!(BitcoinAddress::detect_type("invalid").is_err());
    }
    
    #[test]
    fn test_multiple_address_generation() {
        // 批量生成address测试
        let mut addresses = std::collections::HashSet::new();
        
        for _ in 0..10 {
            let (_secret, public_key) = generate_test_keypair();
            let address = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
            
            // 所有address应该唯一
            assert!(addresses.insert(address.clone()), "address应该唯一: {}", address);
        }
        
        assert_eq!(addresses.len(), 10);
    }
    
    #[test]
    fn test_network_prefix_correctness() {
        let (_secret, public_key) = generate_test_keypair();
        
        // Mainnet prefixes
        let mainnet_legacy = BitcoinAddress::from_public_key_legacy(&public_key, Network::Bitcoin).unwrap();
        assert!(mainnet_legacy.starts_with('1') || mainnet_legacy.starts_with('3'));
        
        let mainnet_segwit = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
        assert!(mainnet_segwit.starts_with("bc1"));
        
        // Testnet prefixes
        let testnet_legacy = BitcoinAddress::from_public_key_legacy(&public_key, Network::Testnet).unwrap();
        assert!(testnet_legacy.starts_with('m') || testnet_legacy.starts_with('n'));
        
        let testnet_segwit = BitcoinAddress::from_public_key_segwit(&public_key, Network::Testnet).unwrap();
        assert!(testnet_segwit.starts_with("tb1"));
    }
    
    #[test]
    fn test_address_validation_consistency() {
        // 生成的address应该能通过validate
        let (_secret, public_key) = generate_test_keypair();
        
        let legacy = BitcoinAddress::from_public_key_legacy(&public_key, Network::Bitcoin).unwrap();
        assert!(BitcoinAddress::validate(&legacy, Network::Bitcoin).unwrap());
        
        let segwit = BitcoinAddress::from_public_key_segwit(&public_key, Network::Bitcoin).unwrap();
        assert!(BitcoinAddress::validate(&segwit, Network::Bitcoin).unwrap());
        
        let taproot = BitcoinAddress::from_public_key_taproot(&public_key, Network::Bitcoin).unwrap();
        assert!(BitcoinAddress::validate(&taproot, Network::Bitcoin).unwrap());
    }
}

