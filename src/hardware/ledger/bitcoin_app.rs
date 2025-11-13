//! Ledger Bitcoin App 集成
//! 
//! 实现与 Ledger Bitcoin 应用的交互

use super::apdu::{ApduClass, ApduCommand, ApduInstruction};
use super::device::LedgerDevice;
use crate::core::errors::WalletError;
use tracing::{debug, info};

/// BIP32 路径
#[derive(Debug, Clone)]
pub struct Bip32Path {
    pub path: Vec<u32>,
}

impl Bip32Path {
    /// 创建新的 BIP32 路径
    pub fn new(path: Vec<u32>) -> Self {
        Self { path }
    }
    
    /// from字符串解析（如 "m/44'/0'/0'/0/0"）
    pub fn from_str(path_str: &str) -> Result<Self, WalletError> {
        // validate路径必须以 "m/" 开头
        if !path_str.starts_with("m/") {
            return Err(WalletError::ValidationError(format!("路径必须以 m/ 开头: {}", path_str)));
        }
        
        let path_without_m = path_str.trim_start_matches("m/");
        
        // 路径必须包含至少一个组件（"m" 和 "m/" 都应该failed）
        if path_without_m.is_empty() {
            return Err(WalletError::ValidationError(format!("路径必须包含至少一个组件: {}", path_str)));
        }
        
        let parts: Vec<&str> = path_without_m.split('/').collect();
        let mut path = Vec::new();
        
        for part in parts {
            if part.is_empty() {
                return Err(WalletError::ValidationError(format!("路径包含空组件: {}", path_str)));
            }
            
            let hardened = part.ends_with('\'') || part.ends_with('h');
            let num_str = part.trim_end_matches('\'').trim_end_matches('h');
            
            if num_str.is_empty() {
                return Err(WalletError::ValidationError(format!("路径组件无效: {}", part)));
            }
            
            let num: u32 = num_str
                .parse()
                .map_err(|_| WalletError::ValidationError(format!("无效的路径: {}", path_str)))?;
            
            let index = if hardened {
                0x80000000 | num
            } else {
                num
            };
            
            path.push(index);
        }
        
        Ok(Self { path })
    }
    
    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.path.len() as u8);
        
        for index in &self.path {
            bytes.extend_from_slice(&index.to_be_bytes());
        }
        
        bytes
    }
}

/// Ledger Bitcoin App
pub struct LedgerBitcoinApp {
    device: LedgerDevice,
}

impl LedgerBitcoinApp {
    /// 创建 Bitcoin App 实例
    pub fn new(device: LedgerDevice) -> Self {
        Self { device }
    }
    
    /// 连接到 Ledger 并初始化 Bitcoin App
    pub fn connect() -> Result<Self, WalletError> {
        let device = LedgerDevice::connect()?;
        Ok(Self::new(device))
    }
    
    /// fetch公钥
    pub fn get_public_key(
        &self,
        path: &Bip32Path,
        display: bool,
    ) -> Result<(Vec<u8>, String), WalletError> {
        info!("fetch Bitcoin 公钥，路径: {:?}", path.path);
        
        let p1 = if display { 0x01 } else { 0x00 };
        let p2 = 0x00; // 返回公钥
        
        let command = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            p1,
            p2,
            path.to_bytes(),
        );
        
        let response = self.device.exchange(&command)?;
        
        if !response.is_success() {
            return Err(WalletError::CryptoError(format!(
                "fetch公钥failed: {}",
                response.error_description()
            )));
        }
        
        // 解析响应
        if response.data.len() < 65 {
            return Err(WalletError::CryptoError("公钥数据不完整".to_string()));
        }
        
        let pub_key_len = response.data[0] as usize;
        if response.data.len() < 1 + pub_key_len {
            return Err(WalletError::CryptoError("公钥数据格式error".to_string()));
        }
        
        let pub_key = response.data[1..1 + pub_key_len].to_vec();
        
        // 解析address
        let mut offset = 1 + pub_key_len;
        let address = if offset < response.data.len() {
            let addr_len = response.data[offset] as usize;
            offset += 1;
            if response.data.len() >= offset + addr_len {
                String::from_utf8_lossy(&response.data[offset..offset + addr_len]).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        debug!("✅ 公钥长度: {}, address: {}", pub_key.len(), address);
        
        Ok((pub_key, address))
    }
    
    /// Sign transaction
    pub fn sign_transaction(
        &self,
        path: &Bip32Path,
        tx_data: &[u8],
    ) -> Result<Vec<u8>, WalletError> {
        info!("Ledger Sign transaction，路径: {:?}", path.path);
        
        // 第一步：发送路径
        let mut data = path.to_bytes();
        data.extend_from_slice(tx_data);
        
        let command = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignTransaction,
            0x00,
            0x00,
            data,
        );
        
        let response = self.device.exchange(&command)?;
        
        if !response.is_success() {
            return Err(WalletError::SigningFailed(format!(
                "Signing failed: {}",
                response.error_description()
            )));
        }
        
        info!("✅ Transaction signed successfully，sign长度: {}", response.data.len());
        
        Ok(response.data)
    }
    
    /// fetch应用版本
    pub fn get_version(&self) -> Result<String, WalletError> {
        let app_info = self.device.get_app_configuration()?;
        Ok(app_info.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bip32_path_parsing() {
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        assert_eq!(path.path.len(), 5);
        assert_eq!(path.path[0], 0x8000002C); // 44'
        assert_eq!(path.path[1], 0x80000000); // 0'
        assert_eq!(path.path[2], 0x80000000); // 0'
        assert_eq!(path.path[3], 0x00000000); // 0
        assert_eq!(path.path[4], 0x00000000); // 0
    }
    
    #[test]
    fn test_bip32_path_serialization() {
        let path = Bip32Path::new(vec![0x8000002C, 0x80000000]);
        let bytes = path.to_bytes();
        
        assert_eq!(bytes[0], 2); // 路径长度
        assert_eq!(&bytes[1..5], &[0x80, 0x00, 0x00, 0x2C]); // 44'
        assert_eq!(&bytes[5..9], &[0x80, 0x00, 0x00, 0x00]); // 0'
    }
    
    #[test]
    fn test_taproot_path() {
        // BIP86: m/86'/0'/0'/0/0
        let path = Bip32Path::from_str("m/86'/0'/0'/0/0").unwrap();
        
        assert_eq!(path.path[0], 0x80000056); // 86'
    }
    
    // 需要实际设备的测试
    
    #[test]
    #[ignore]
    fn test_get_public_key_from_device() {
        let app = LedgerBitcoinApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        let (pubkey, address) = app.get_public_key(&path, false).unwrap();
        
        assert!(!pubkey.is_empty());
        assert!(!address.is_empty());
    }
}


