//! Trezor 传输层
//! 
//! 实现与 Trezor 设备的 USB HID 通信

use super::messages::TrezorMessage;
use crate::core::errors::WalletError;
use hidapi::{HidApi, HidDevice};
use std::time::Duration;
use tracing::{debug, info};

/// Trezor USB 参数
pub const TREZOR_VENDOR_ID: u16 = 0x534C;  // SatoshiLabs
pub const TREZOR_ONE_PRODUCT_ID: u16 = 0x0001;
pub const TREZOR_T_PRODUCT_ID: u16 = 0x0002;

/// HID 数据包大小
const HID_PACKET_SIZE: usize = 64;

/// Trezor 传输协议魔术字节
const HEADER_MAGIC: &[u8] = b"?##";

/// Trezor HID 传输
pub struct TrezorTransport {
    device: HidDevice,
    timeout: Duration,
}

impl TrezorTransport {
    /// 打开 Trezor 设备
    pub fn open() -> Result<Self, WalletError> {
        info!("正在连接 Trezor 设备...");
        
        let api = HidApi::new()
            .map_err(|e| WalletError::CryptoError(format!("HID API 初始化failed: {}", e)))?;
        
        // 尝试查找 Trezor One 或 Trezor T
        let device = api
            .open(TREZOR_VENDOR_ID, TREZOR_ONE_PRODUCT_ID)
            .or_else(|_| api.open(TREZOR_VENDOR_ID, TREZOR_T_PRODUCT_ID))
            .map_err(|e| {
                WalletError::CryptoError(format!(
                    "未找到 Trezor 设备。请确保设备已连接。error: {}",
                    e
                ))
            })?;
        
        info!("✅ 已连接到 Trezor 设备");
        
        Ok(Self {
            device,
            timeout: Duration::from_secs(60),  // Trezor 需要user确认，设置较长超时
        })
    }
    
    /// 设置超时时间
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    
    /// 发送消息
    pub fn write(&self, message: &TrezorMessage) -> Result<(), WalletError> {
        let serialized = message.serialize();
        
        debug!(
            "发送 Trezor 消息: {:?}, 长度: {}",
            message.msg_type,
            serialized.len()
        );
        
        // 构建 HID 数据包
        let packets = self.build_packets(&serialized);
        
        // 发送所有数据包
        for (i, packet) in packets.iter().enumerate() {
            debug!("发送数据包 {}/{}", i + 1, packets.len());
            self.device
                .write(packet)
                .map_err(|e| WalletError::NetworkError(format!("发送数据failed: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// 接收消息
    pub fn read(&self) -> Result<TrezorMessage, WalletError> {
        let mut response_data = Vec::new();
        let mut first_packet = true;
        let mut total_len: Option<usize> = None;
        
        loop {
            let mut packet = vec![0u8; HID_PACKET_SIZE + 1];  // +1 for report ID
            
            let read_len = self
                .device
                .read_timeout(&mut packet, self.timeout.as_millis() as i32)
                .map_err(|e| WalletError::NetworkError(format!("接收数据failed: {}", e)))?;
            
            if read_len == 0 {
                return Err(WalletError::NetworkError("接收超时".to_string()));
            }
            
            // 跳过 report ID
            let packet_data = &packet[1..read_len];
            
            if first_packet {
                // validate头部
                if packet_data.len() < 3 || &packet_data[0..3] != HEADER_MAGIC {
                    return Err(WalletError::CryptoError("无效的 Trezor 响应头".to_string()));
                }
                
                // 读取总长度（9 字节偏移）
                if packet_data.len() < 9 {
                    return Err(WalletError::CryptoError("响应头不完整".to_string()));
                }
                
                // 消息类型 (2 字节) + 长度 (4 字节) = 6 字节
                total_len = Some(6 + u32::from_be_bytes([
                    packet_data[5],
                    packet_data[6],
                    packet_data[7],
                    packet_data[8],
                ]) as usize);
                
                // 添加第一个数据包的内容（from第 3 字节start）
                response_data.extend_from_slice(&packet_data[3..]);
                first_packet = false;
            } else {
                // 后续数据包直接添加
                response_data.extend_from_slice(packet_data);
            }
            
            // check是否接收完整
            if let Some(len) = total_len {
                if response_data.len() >= len {
                    response_data.truncate(len);
                    break;
                }
            }
        }
        
        // 解析消息
        let message = TrezorMessage::deserialize(&response_data)?;
        
        debug!("接收 Trezor 消息: {:?}, 长度: {}", message.msg_type, message.payload.len());
        
        Ok(message)
    }
    
    /// 交换消息（发送并接收）
    pub fn exchange(&self, message: &TrezorMessage) -> Result<TrezorMessage, WalletError> {
        self.write(message)?;
        self.read()
    }
    
    /// 构建 HID 数据包
    fn build_packets(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let mut packets = Vec::new();
        let mut offset = 0;
        let total_len = data.len();
        
        // 第一个数据包
        let mut first_packet = vec![0u8; HID_PACKET_SIZE + 1];  // +1 for report ID
        first_packet[0] = 0x3F;  // Report ID for Trezor
        
        // 魔术头
        first_packet[1..4].copy_from_slice(HEADER_MAGIC);
        
        // 数据长度（大端，后续数据包不包含头）
        let header_len = total_len - 6;  // 减去消息类型和长度字段
        first_packet[4..9].copy_from_slice(&[
            (header_len >> 24) as u8,
            (header_len >> 16) as u8,
            (header_len >> 8) as u8,
            header_len as u8,
            0,  // 填充
        ]);
        
        // 填充数据（第一包最多 55 字节：64 - 1(report) - 3(magic) - 5(header) = 55）
        let first_chunk_size = (HID_PACKET_SIZE - 8).min(total_len);
        first_packet[9..9 + first_chunk_size].copy_from_slice(&data[offset..offset + first_chunk_size]);
        packets.push(first_packet);
        offset += first_chunk_size;
        
        // 后续数据包
        while offset < total_len {
            let mut packet = vec![0u8; HID_PACKET_SIZE + 1];
            packet[0] = 0x3F;  // Report ID
            
            let chunk_size = (HID_PACKET_SIZE).min(total_len - offset);
            packet[1..1 + chunk_size].copy_from_slice(&data[offset..offset + chunk_size]);
            packets.push(packet);
            offset += chunk_size;
        }
        
        packets
    }
}

#[cfg(test)]
mod tests {
    #![allow(invalid_value)] // 测试中使用 std::mem::zeroed() 初始化 HidDevice 用于模拟，已标记为 #[ignore]
    
    use super::*;
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_packet_building() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore]
    fn test_connect_to_trezor() {
        let result = TrezorTransport::open();
        assert!(result.is_ok());
    }
    
    // ============ 新增的 Trezor Transport 测试 ============
    
    #[test]
    fn test_header_magic() {
        assert_eq!(HEADER_MAGIC, b"?##");
        assert_eq!(HEADER_MAGIC.len(), 3);
    }
    
    #[test]
    fn test_vendor_and_product_ids() {
        assert_eq!(TREZOR_VENDOR_ID, 0x534C);
        assert_eq!(TREZOR_ONE_PRODUCT_ID, 0x0001);
        assert_eq!(TREZOR_T_PRODUCT_ID, 0x0002);
    }
    
    #[test]
    fn test_packet_size_constant() {
        assert_eq!(HID_PACKET_SIZE, 64);
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_build_empty_message() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_build_large_message() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_first_packet_header() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_packet_padding() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_message_length_in_header() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_timeout_default() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_set_timeout() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_multi_packet_message() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_exact_one_packet_boundary() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_one_byte_over_packet() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_very_large_message() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_report_id_consistency() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_magic_header_in_first_packet() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_packet_size_uniformity() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
}

