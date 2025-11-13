//! Trezor Bitcoin App 集成
//! 
//! 实现与 Trezor Bitcoin 功能的交互

use super::device::TrezorDevice;
use super::messages::{
    encode_bip32_path, encode_bool_field, encode_string_field, MessageType, TrezorMessage,
};
use crate::core::errors::WalletError;
use crate::hardware::ledger::bitcoin_app::Bip32Path;  // 复用 Ledger 的 BIP32 路径
use tracing::{debug, info};

/// Trezor Bitcoin App
pub struct TrezorBitcoinApp {
    device: TrezorDevice,
}

impl TrezorBitcoinApp {
    /// 创建 Bitcoin App 实例
    pub fn new(device: TrezorDevice) -> Self {
        Self { device }
    }
    
    /// 连接到 Trezor 并初始化 Bitcoin App
    pub fn connect() -> Result<Self, WalletError> {
        let device = TrezorDevice::connect()?;
        Ok(Self::new(device))
    }
    
    /// fetch公钥和address
    pub fn get_address(
        &self,
        path: &Bip32Path,
        show_display: bool,
    ) -> Result<String, WalletError> {
        info!("fetch Bitcoin address，路径: {:?}", path.path);
        
        // 构建 GetAddress 消息
        let mut payload = Vec::new();
        
        // address_n: repeated uint32 (field 1)
        payload.extend_from_slice(&encode_bip32_path(&path.path));
        
        // coin_name: string (field 2)
        payload.extend_from_slice(&encode_string_field(2, "Bitcoin"));
        
        // show_display: bool (field 3)
        if show_display {
            payload.extend_from_slice(&encode_bool_field(3, true));
        }
        
        let msg = TrezorMessage::new(MessageType::GetAddress, payload);
        let mut response = self.device.call(&msg)?;
        
        // 处理可能的 ButtonRequest
        while response.msg_type == MessageType::ButtonRequest {
            response = self.device.handle_button_request()?;
        }
        
        if response.msg_type != MessageType::Address {
            return Err(WalletError::CryptoError(format!(
                "期待 Address 消息，收到 {:?}",
                response.msg_type
            )));
        }
        
        // 解析address（简化版）
        let address = Self::parse_address(&response.payload)?;
        
        debug!("✅ address: {}", address);
        
        Ok(address)
    }
    
    /// fetch扩展公钥
    pub fn get_public_key(
        &self,
        path: &Bip32Path,
    ) -> Result<Vec<u8>, WalletError> {
        info!("fetch Bitcoin 公钥，路径: {:?}", path.path);
        
        // 构建 GetPublicKey 消息
        let payload = encode_bip32_path(&path.path);
        let msg = TrezorMessage::new(MessageType::GetPublicKey, payload);
        
        let mut response = self.device.call(&msg)?;
        
        // 处理可能的 ButtonRequest
        while response.msg_type == MessageType::ButtonRequest {
            response = self.device.handle_button_request()?;
        }
        
        if response.msg_type != MessageType::PublicKey {
            return Err(WalletError::CryptoError(format!(
                "期待 PublicKey 消息，收到 {:?}",
                response.msg_type
            )));
        }
        
        // 解析公钥（简化版）
        let pubkey = Self::parse_public_key(&response.payload)?;
        
        debug!("✅ 公钥长度: {}", pubkey.len());
        
        Ok(pubkey)
    }
    
    /// 解析address消息
    fn parse_address(payload: &[u8]) -> Result<String, WalletError> {
        // 简化的 Protobuf 解析
        // 实际生产代码应使用 prost 生成的代码
        
        // 查找字符串字段（field 1，wire type 2）
        for i in 0..payload.len() {
            if payload[i] == 0x0A {  // field 1, wire_type 2: (1 << 3) | 2 = 0x0A
                if i + 1 < payload.len() {
                    let len = payload[i + 1] as usize;
                    if i + 2 + len <= payload.len() {
                        let addr_bytes = &payload[i + 2..i + 2 + len];
                        return Ok(String::from_utf8_lossy(addr_bytes).to_string());
                    }
                }
            }
        }
        
        Err(WalletError::CryptoError("无法解析address".to_string()))
    }
    
    /// 解析公钥消息
    fn parse_public_key(payload: &[u8]) -> Result<Vec<u8>, WalletError> {
        // 查找字节字段（field 1，wire type 2）
        for i in 0..payload.len() {
            if payload[i] == 0x0A {  // field 1, wire_type 2
                if i + 1 < payload.len() {
                    let len = payload[i + 1] as usize;
                    if i + 2 + len <= payload.len() {
                        return Ok(payload[i + 2..i + 2 + len].to_vec());
                    }
                }
            }
        }
        
        Err(WalletError::CryptoError("无法解析公钥".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bip32_path() {
        // BIP44: m/44'/0'/0'/0/0
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        assert_eq!(path.path.len(), 5);
    }
    
    #[test]
    #[ignore]
    fn test_get_address() {
        let app = TrezorBitcoinApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        let address = app.get_address(&path, false).unwrap();
        assert!(!address.is_empty());
    }
    
    #[test]
    #[ignore]
    fn test_get_public_key() {
        let app = TrezorBitcoinApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        let pubkey = app.get_public_key(&path).unwrap();
        assert!(!pubkey.is_empty());
    }
}

