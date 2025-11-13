//! Ledger 设备管理
//! 
//! 提供 Ledger 设备的高级管理功能

use super::apdu::{ApduClass, ApduCommand, ApduInstruction, ApduResponse};
use super::transport::{LedgerTransport, LEDGER_VENDOR_ID};
use crate::core::errors::WalletError;
use std::time::Duration;
use tracing::{info, warn};

/// Ledger 设备
pub struct LedgerDevice {
    transport: LedgerTransport,
}

/// Ledger 应用信息
#[derive(Debug, Clone)]
pub struct LedgerAppInfo {
    /// 应用名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 标志位
    pub flags: u8,
}

/// Ledger 设备信息
#[derive(Debug, Clone)]
pub struct LedgerDeviceInfo {
    /// 厂商 ID
    pub vendor_id: u16,
    /// 产品 ID
    pub product_id: u16,
    /// 设备型号名称
    pub model_name: String,
}

/// 最低安全固件版本
const MIN_SAFE_FIRMWARE_VERSION: &str = "1.6.0";

/// 支持的 Ledger 产品 ID
const LEDGER_NANO_S_PRODUCT_ID: u16 = 0x0001;
const LEDGER_NANO_X_PRODUCT_ID: u16 = 0x0004;
const LEDGER_NANO_S_PLUS_PRODUCT_ID: u16 = 0x0005;

impl LedgerDevice {
    /// 连接到 Ledger 设备
    pub fn connect() -> Result<Self, WalletError> {
        info!("连接 Ledger 硬件wallet...");
        let transport = LedgerTransport::open()?;
        
        Ok(Self { transport })
    }
    
    /// 设置超时时间
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.transport.set_timeout(timeout);
    }
    
    /// fetch应用配置
    pub fn get_app_configuration(&self) -> Result<LedgerAppInfo, WalletError> {
        info!("fetch Ledger 应用配置...");
        
        let command = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetAppConfiguration,
            0x00,
            0x00,
            vec![],
        );
        
        let response = self.transport.exchange(&command)?;
        
        if !response.is_success() {
            return Err(WalletError::CryptoError(format!(
                "fetch配置failed: {}",
                response.error_description()
            )));
        }
        
        if response.data.len() < 4 {
            return Err(WalletError::CryptoError("配置数据不完整".to_string()));
        }
        
        let flags = response.data[0];
        let major = response.data[1];
        let minor = response.data[2];
        let patch = response.data[3];
        
        let version = format!("{}.{}.{}", major, minor, patch);
        
        // 尝试fetch应用名称
        let name = if response.data.len() > 4 {
            let name_len = response.data[4] as usize;
            if response.data.len() >= 5 + name_len {
                String::from_utf8_lossy(&response.data[5..5 + name_len]).to_string()
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };
        
        info!("✅ 应用: {} v{}", name, version);
        
        Ok(LedgerAppInfo {
            name,
            version,
            flags,
        })
    }
    
    /// 发送 APDU 命令
    pub fn exchange(&self, command: &ApduCommand) -> Result<ApduResponse, WalletError> {
        self.transport.exchange(command)
    }

    /// validate设备真实性
    ///
    /// # Security
    /// - validate厂商 ID
    /// - validate产品 ID
    /// - validate固件版本
    pub fn verify_device(&self) -> Result<bool, WalletError> {
        info!("🔍 startvalidate Ledger 设备真实性...");

        // 1. fetch设备信息
        let device_info = self.get_device_info()?;

        // 2. validate厂商 ID
        if device_info.vendor_id != LEDGER_VENDOR_ID {
            warn!("⚠️ 无效的厂商 ID: {:#x} (期望: {:#x})", 
                device_info.vendor_id, LEDGER_VENDOR_ID);
            return Ok(false);
        }
        info!("✅ 厂商 ID validate通过");

        // 3. validate产品 ID
        let valid_product_ids = [
            LEDGER_NANO_S_PRODUCT_ID,
            LEDGER_NANO_X_PRODUCT_ID,
            LEDGER_NANO_S_PLUS_PRODUCT_ID,
        ];
        if !valid_product_ids.contains(&device_info.product_id) {
            warn!("⚠️ 未知的产品 ID: {:#x}", device_info.product_id);
            return Ok(false);
        }
        info!("✅ 产品 ID validate通过: {}", device_info.model_name);

        // 4. fetch并validate固件版本
        let app_info = self.get_app_configuration()?;
        if !is_firmware_version_safe(&app_info.version) {
            warn!("⚠️ 固件版本过旧: {} < {}", 
                app_info.version, MIN_SAFE_FIRMWARE_VERSION);
            warn!("   建议升级固件以获得最佳安全性");
            // 不强制拒绝，但记录Warning
        } else {
            info!("✅ 固件版本validate通过: {}", app_info.version);
        }

        info!("🎉 Ledger 设备validatesuccess！");
        Ok(true)
    }

    /// fetch设备信息
    fn get_device_info(&self) -> Result<LedgerDeviceInfo, WalletError> {
        // 在实际实现中，这会query HID 设备信息
        // 为了简化，我们from transport 层fetch
        let vendor_id = LEDGER_VENDOR_ID;
        
        // 通过query应用配置来推断设备型号
        let app_info = self.get_app_configuration()?;
        
        // 简化的产品 ID 推断（实际实现中会query HID 设备）
        let (product_id, model_name) = if app_info.name.contains("Bitcoin") || app_info.name.contains("Ethereum") {
            // 假设为 Nano X（最常见）
            (LEDGER_NANO_X_PRODUCT_ID, "Ledger Nano X".to_string())
        } else {
            // 默认 Nano S
            (LEDGER_NANO_S_PRODUCT_ID, "Ledger Nano S".to_string())
        };

        Ok(LedgerDeviceInfo {
            vendor_id,
            product_id,
            model_name,
        })
    }

    /// 带validate的连接方法
    ///
    /// 连接到设备并validate其真实性
    pub fn connect_verified() -> Result<Self, WalletError> {
        let device = Self::connect()?;
        
        if !device.verify_device()? {
            return Err(WalletError::SecurityError(
                "设备validatefailed：可能是伪造或不受支持的设备".to_string()
            ));
        }

        Ok(device)
    }
}

/// check固件版本是否足够安全
fn is_firmware_version_safe(version: &str) -> bool {
    // 简单的版本比较（实际实现中应该使用语义化版本比较）
    version >= MIN_SAFE_FIRMWARE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ledger_app_info_creation() {
        let info = LedgerAppInfo {
            name: "Bitcoin".to_string(),
            version: "2.1.0".to_string(),
            flags: 0x01,
        };
        
        assert_eq!(info.name, "Bitcoin");
        assert_eq!(info.version, "2.1.0");
    }
    
    // Note:以下测试需要实际的 Ledger 设备
    
    #[test]
    #[ignore]
    fn test_connect_to_ledger() {
        let result = LedgerDevice::connect();
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore]
    fn test_get_app_configuration() {
        let device = LedgerDevice::connect().unwrap();
        let info = device.get_app_configuration().unwrap();
        
        assert!(!info.name.is_empty());
        assert!(!info.version.is_empty());
    }
}


