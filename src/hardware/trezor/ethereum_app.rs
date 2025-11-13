//! Trezor Ethereum App 集成
//! 
//! 实现与 Trezor Ethereum 功能的交互

use super::device::TrezorDevice;
use super::messages::{
    encode_bip32_path, encode_bool_field, MessageType, TrezorMessage,
};
use crate::core::errors::WalletError;
use crate::hardware::ledger::bitcoin_app::Bip32Path;
use tracing::{debug, info};

/// Trezor Ethereum App
pub struct TrezorEthereumApp {
    device: TrezorDevice,
}

impl TrezorEthereumApp {
    /// 创建 Ethereum App 实例
    pub fn new(device: TrezorDevice) -> Self {
        Self { device }
    }
    
    /// 连接到 Trezor 并初始化 Ethereum App
    pub fn connect() -> Result<Self, WalletError> {
        let device = TrezorDevice::connect()?;
        Ok(Self::new(device))
    }
    
    /// fetch以太坊address
    pub fn get_address(
        &self,
        path: &Bip32Path,
        show_display: bool,
    ) -> Result<String, WalletError> {
        info!("fetch Ethereum address，路径: {:?}", path.path);
        
        // 构建 EthereumGetAddress 消息
        let mut payload = Vec::new();
        
        // address_n: repeated uint32 (field 1)
        payload.extend_from_slice(&encode_bip32_path(&path.path));
        
        // show_display: bool (field 2)
        if show_display {
            payload.extend_from_slice(&encode_bool_field(2, true));
        }
        
        let msg = TrezorMessage::new(MessageType::EthereumGetAddress, payload);
        let mut response = self.device.call(&msg)?;
        
        // 处理可能的 ButtonRequest
        while response.msg_type == MessageType::ButtonRequest {
            response = self.device.handle_button_request()?;
        }
        
        if response.msg_type != MessageType::EthereumAddress {
            return Err(WalletError::CryptoError(format!(
                "期待 EthereumAddress 消息，收到 {:?}",
                response.msg_type
            )));
        }
        
        // 解析address
        let address = Self::parse_address(&response.payload)?;
        
        debug!("✅ Ethereum address: {}", address);
        
        Ok(address)
    }
    
    /// 解析以太坊address消息
    fn parse_address(payload: &[u8]) -> Result<String, WalletError> {
        // 简化的 Protobuf 解析
        // 查找字节字段（field 2，address，wire type 2）
        for i in 0..payload.len() {
            if payload[i] == 0x12 {  // field 2, wire_type 2: (2 << 3) | 2 = 0x12
                if i + 1 < payload.len() {
                    let len = payload[i + 1] as usize;
                    if i + 2 + len <= payload.len() {
                        let addr_bytes = &payload[i + 2..i + 2 + len];
                        // Ethereum address是 20 字节的十六进制
                        let addr_hex = hex::encode(addr_bytes);
                        return Ok(format!("0x{}", addr_hex));
                    }
                }
            }
        }
        
        Err(WalletError::CryptoError("无法解析以太坊address".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ethereum_path() {
        // BIP44: m/44'/60'/0'/0/0
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        assert_eq!(path.path.len(), 5);
        assert_eq!(path.path[1], 0x8000003C); // 60'
    }
    
    #[test]
    #[ignore]
    fn test_get_ethereum_address() {
        let app = TrezorEthereumApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        let address = app.get_address(&path, false).unwrap();
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42); // 0x + 40 hex chars
    }
}

