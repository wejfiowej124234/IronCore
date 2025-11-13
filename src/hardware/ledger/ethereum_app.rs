//! Ledger Ethereum App 集成
//! 
//! 实现与 Ledger Ethereum 应用的交互

use super::apdu::{ApduClass, ApduCommand, ApduInstruction};
use super::bitcoin_app::Bip32Path;
use super::device::LedgerDevice;
use crate::core::errors::WalletError;
use tracing::{debug, info};

/// Ledger Ethereum App
pub struct LedgerEthereumApp {
    device: LedgerDevice,
}

impl LedgerEthereumApp {
    /// 创建 Ethereum App 实例
    pub fn new(device: LedgerDevice) -> Self {
        Self { device }
    }
    
    /// 连接到 Ledger 并初始化 Ethereum App
    pub fn connect() -> Result<Self, WalletError> {
        let device = LedgerDevice::connect()?;
        Ok(Self::new(device))
    }
    
    /// fetch以太坊address
    pub fn get_address(
        &self,
        path: &Bip32Path,
        display: bool,
    ) -> Result<(Vec<u8>, String), WalletError> {
        info!("fetch Ethereum address，路径: {:?}", path.path);
        
        let p1 = if display { 0x01 } else { 0x00 };
        let p2 = 0x00; // 返回address
        
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
                "fetchaddressfailed: {}",
                response.error_description()
            )));
        }
        
        // 解析响应
        if response.data.len() < 65 {
            return Err(WalletError::CryptoError("address数据不完整".to_string()));
        }
        
        let pub_key_len = response.data[0] as usize;
        let pub_key = response.data[1..1 + pub_key_len].to_vec();
        
        // 解析以太坊address
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
    
    /// sign以太坊transaction
    pub fn sign_transaction(
        &self,
        path: &Bip32Path,
        tx_data: &[u8],
    ) -> Result<(u8, Vec<u8>, Vec<u8>), WalletError> {
        info!("Ledger sign Ethereum transaction，路径: {:?}", path.path);
        
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
        
        // 解析sign (v, r, s)
        if response.data.len() < 65 {
            return Err(WalletError::SigningFailed("sign数据不完整".to_string()));
        }
        
        let v = response.data[0];
        let r = response.data[1..33].to_vec();
        let s = response.data[33..65].to_vec();
        
        info!("✅ signsuccess: v={}, r_len={}, s_len={}", v, r.len(), s.len());
        
        Ok((v, r, s))
    }
    
    /// sign个人消息（EIP-191）
    pub fn sign_personal_message(
        &self,
        path: &Bip32Path,
        message: &[u8],
    ) -> Result<(u8, Vec<u8>, Vec<u8>), WalletError> {
        info!("Ledger sign个人消息，长度: {}", message.len());
        
        let mut data = path.to_bytes();
        
        // 添加消息长度
        let msg_len_bytes = (message.len() as u32).to_be_bytes();
        data.extend_from_slice(&msg_len_bytes);
        
        // 添加消息
        data.extend_from_slice(message);
        
        let command = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignMessage,
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
        
        // 解析sign
        if response.data.len() < 65 {
            return Err(WalletError::SigningFailed("sign数据不完整".to_string()));
        }
        
        let v = response.data[0];
        let r = response.data[1..33].to_vec();
        let s = response.data[33..65].to_vec();
        
        info!("✅ 消息signsuccess");
        
        Ok((v, r, s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bip32_path_standard() {
        // BIP44: m/44'/60'/0'/0/0 (Ethereum)
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        assert_eq!(path.path.len(), 5);
        assert_eq!(path.path[0], 0x8000002C); // 44'
        assert_eq!(path.path[1], 0x8000003C); // 60' (Ethereum)
        assert_eq!(path.path[2], 0x80000000); // 0'
        assert_eq!(path.path[3], 0x00000000); // 0
        assert_eq!(path.path[4], 0x00000000); // 0
    }
    
    #[test]
    fn test_bip32_path_bytes() {
        let path = Bip32Path::new(vec![0x8000002C, 0x80000000]);
        let bytes = path.to_bytes();
        
        assert_eq!(bytes[0], 2); // 路径深度
        // 第一个索引: 44'
        assert_eq!(bytes[1], 0x80);
        assert_eq!(bytes[2], 0x00);
        assert_eq!(bytes[3], 0x00);
        assert_eq!(bytes[4], 0x2C);
    }
    
    // 需要实际设备的测试
    
    #[test]
    #[ignore]
    fn test_get_ethereum_address() {
        let app = LedgerEthereumApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        let (pubkey, address) = app.get_address(&path, false).unwrap();
        
        assert_eq!(pubkey.len(), 65);
        assert!(address.starts_with("0x"));
    }
}


