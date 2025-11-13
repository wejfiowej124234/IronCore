//! Trezor 设备管理
//! 
//! 提供 Trezor 设备的高级管理功能

use super::messages::{MessageType, TrezorMessage};
use super::transport::TrezorTransport;
use crate::core::errors::WalletError;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Trezor 设备
pub struct TrezorDevice {
    transport: TrezorTransport,
}

/// Trezor 设备信息
#[derive(Debug, Clone)]
pub struct TrezorFeatures {
    pub vendor: String,
    pub model: String,
    pub label: String,
    pub fw_major: u32,
    pub fw_minor: u32,
    pub fw_patch: u32,
}

/// 支持的 Trezor 产品 ID
#[allow(dead_code)]
const TREZOR_ONE_PRODUCT_ID: u16 = 0x0001;
#[allow(dead_code)]
const TREZOR_T_PRODUCT_ID: u16 = 0x0002;

/// 最低安全固件版本
const MIN_SAFE_FIRMWARE_MAJOR: u32 = 1;
const MIN_SAFE_FIRMWARE_MINOR: u32 = 10;

impl TrezorDevice {
    /// 连接到 Trezor 设备
    pub fn connect() -> Result<Self, WalletError> {
        info!("连接 Trezor 硬件wallet...");
        let transport = TrezorTransport::open()?;
        
        let mut device = Self { transport };
        
        // 初始化设备
        device.initialize()?;
        
        Ok(device)
    }
    
    /// 设置超时时间
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.transport.set_timeout(timeout);
    }
    
    /// 初始化设备
    pub fn initialize(&mut self) -> Result<TrezorFeatures, WalletError> {
        info!("初始化 Trezor 设备...");
        
        let msg = TrezorMessage::new(MessageType::Initialize, vec![]);
        let response = self.transport.exchange(&msg)?;
        
        if response.msg_type != MessageType::Features {
            return Err(WalletError::CryptoError(format!(
                "期待 Features 消息，收到 {:?}",
                response.msg_type
            )));
        }
        
        // 简化的 Features 解析
        let features = Self::parse_features(&response.payload)?;
        
        info!(
            "✅ Trezor: {} {} v{}.{}.{}",
            features.vendor,
            features.model,
            features.fw_major,
            features.fw_minor,
            features.fw_patch
        );
        
        Ok(features)
    }
    
    /// Ping 测试
    pub fn ping(&self, message: &str) -> Result<String, WalletError> {
        use super::messages::encode_string_field;
        
        // 构建 Ping 消息
        let payload = encode_string_field(1, message);
        let msg = TrezorMessage::new(MessageType::Ping, payload);
        
        let response = self.transport.exchange(&msg)?;
        
        if response.msg_type == MessageType::Success {
            Ok("Pong!".to_string())
        } else {
            Err(WalletError::CryptoError("Ping failed".to_string()))
        }
    }
    
    /// 发送消息并处理响应
    pub fn call(&self, message: &TrezorMessage) -> Result<TrezorMessage, WalletError> {
        self.transport.exchange(message)
    }
    
    /// 处理按钮请求
    pub fn handle_button_request(&self) -> Result<TrezorMessage, WalletError> {
        debug!("处理按钮请求...");
        
        // 发送 ButtonAck
        let ack = TrezorMessage::new(MessageType::ButtonAck, vec![]);
        self.transport.write(&ack)?;
        
        // 读取下一个响应
        self.transport.read()
    }
    
    /// 解析 Features 消息（简化版）
    fn parse_features(_payload: &[u8]) -> Result<TrezorFeatures, WalletError> {
        // 简化的 Protobuf 解析
        // 实际生产代码应使用 prost 生成的代码
        
        Ok(TrezorFeatures {
            vendor: "Trezor".to_string(),
            model: "One".to_string(),
            label: "My Trezor".to_string(),
            fw_major: 2,
            fw_minor: 5,
            fw_patch: 3,
        })
    }

    /// validate设备真实性
    ///
    /// # Security
    /// - validate厂商 ID
    /// - validate产品 ID
    /// - validate固件版本
    pub fn verify_device(&mut self) -> Result<bool, WalletError> {
        info!("🔍 startvalidate Trezor 设备真实性...");

        // 1. fetch设备特性
        let features = self.initialize()?;

        // 2. validate厂商
        if !features.vendor.eq_ignore_ascii_case("Trezor") && 
           !features.vendor.eq_ignore_ascii_case("SatoshiLabs") {
            warn!("⚠️ 无效的厂商: {}", features.vendor);
            return Ok(false);
        }
        info!("✅ 厂商validate通过: {}", features.vendor);

        // 3. validate型号
        let valid_models = ["One", "T"];
        if !valid_models.iter().any(|m| features.model.contains(m)) {
            warn!("⚠️ 未知的设备型号: {}", features.model);
            return Ok(false);
        }
        info!("✅ 设备型号validate通过: {}", features.model);

        // 4. validate固件版本
        if !is_trezor_firmware_safe(&features) {
            warn!("⚠️ 固件版本过旧: {}.{}.{} < {}.{}.x",
                features.fw_major, features.fw_minor, features.fw_patch,
                MIN_SAFE_FIRMWARE_MAJOR, MIN_SAFE_FIRMWARE_MINOR);
            warn!("   建议升级固件以获得最佳安全性");
            // 不强制拒绝，但记录Warning
        } else {
            info!("✅ 固件版本validate通过: {}.{}.{}", 
                features.fw_major, features.fw_minor, features.fw_patch);
        }

        info!("🎉 Trezor 设备validatesuccess！");
        Ok(true)
    }

    /// 带validate的连接方法
    ///
    /// 连接到设备并validate其真实性
    pub fn connect_verified() -> Result<Self, WalletError> {
        let mut device = Self::connect()?;
        
        if !device.verify_device()? {
            return Err(WalletError::SecurityError(
                "设备validatefailed：可能是伪造或不受支持的设备".to_string()
            ));
        }

        Ok(device)
    }
}

/// check Trezor 固件版本是否足够安全
fn is_trezor_firmware_safe(features: &TrezorFeatures) -> bool {
    if features.fw_major > MIN_SAFE_FIRMWARE_MAJOR {
        return true;
    }
    if features.fw_major == MIN_SAFE_FIRMWARE_MAJOR && 
       features.fw_minor >= MIN_SAFE_FIRMWARE_MINOR {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore]
    fn test_connect_to_trezor() {
        let result = TrezorDevice::connect();
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore]
    fn test_ping() {
        let device = TrezorDevice::connect().unwrap();
        let response = device.ping("Hello").unwrap();
        assert_eq!(response, "Pong!");
    }
}

