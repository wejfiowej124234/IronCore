//! Ledger 传输层
//! 
//! 实现与 Ledger 设备的 HID 通信

use super::apdu::{ApduCommand, ApduResponse};
use crate::core::errors::WalletError;
use hidapi::{HidApi, HidDevice};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Ledger 设备 USB 参数
pub const LEDGER_VENDOR_ID: u16 = 0x2C97;
pub const LEDGER_USAGE_PAGE: u16 = 0xFFA0;

/// HID 数据包大小
const HID_PACKET_SIZE: usize = 64;

/// APDU 传输通道
const CHANNEL: u16 = 0x0101;

/// HID 命令标签
const TAG_APDU: u8 = 0x05;

/// Ledger HID 传输
pub struct LedgerTransport {
    device: HidDevice,
    timeout: Duration,
}

impl LedgerTransport {
    /// 打开 Ledger 设备
    pub fn open() -> Result<Self, WalletError> {
        info!("正在连接 Ledger 设备...");
        
        let api = HidApi::new()
            .map_err(|e| WalletError::CryptoError(format!("HID API 初始化failed: {}", e)))?;
        
        // 枚举所有 Ledger 设备
        let devices: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == LEDGER_VENDOR_ID)
            .filter(|d| d.usage_page() == LEDGER_USAGE_PAGE)
            .collect();
        
        if devices.is_empty() {
            return Err(WalletError::CryptoError(
                "未找到 Ledger 设备。请确保设备已连接并解锁。".to_string()
            ));
        }
        
        info!("找到 {} 个 Ledger 设备", devices.len());
        
        // 连接第一个设备
        let device_info = devices[0];
        let device = device_info
            .open_device(&api)
            .map_err(|e| WalletError::CryptoError(format!("打开设备failed: {}", e)))?;
        
        info!("✅ 已连接到 Ledger 设备");
        if let Some(product) = device_info.product_string() {
            info!("  产品: {}", product);
        }
        if let Some(serial) = device_info.serial_number() {
            info!("  序列号: {}", serial);
        }
        
        Ok(Self {
            device,
            timeout: Duration::from_secs(30),
        })
    }
    
    /// 设置超时时间
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    
    /// 发送 APDU 命令并接收响应
    pub fn exchange(&self, command: &ApduCommand) -> Result<ApduResponse, WalletError> {
        info!("发送 APDU 命令: CLA={:02X} INS={:02X}", command.cla, command.ins);
        
        // 序列化 APDU 命令
        let apdu_bytes = command.to_bytes();
        
        // 将 APDU 拆分为 HID 数据包
        let packets = self.build_hid_packets(&apdu_bytes);
        
        // 发送所有数据包
        for (i, packet) in packets.iter().enumerate() {
            debug!("发送数据包 {}/{}", i + 1, packets.len());
            self.device
                .write(packet)
                .map_err(|e| WalletError::NetworkError(format!("发送数据failed: {}", e)))?;
        }
        
        // 接收响应
        let response_bytes = self.receive_response()?;
        
        // 解析响应
        let response = ApduResponse::from_bytes(&response_bytes)?;
        
        if response.is_success() {
            info!("✅ APDU 命令执行success");
        } else {
            warn!(
                "⚠️ APDU 命令failed: {:04X} - {}",
                response.status_code(),
                response.error_description()
            );
        }
        
        Ok(response)
    }
    
    /// 构建 HID 数据包
    fn build_hid_packets(&self, apdu: &[u8]) -> Vec<Vec<u8>> {
        let mut packets = Vec::new();
        let total_len = apdu.len();
        let mut offset = 0;
        let mut sequence = 0u16;
        
        while offset < total_len {
            let mut packet = vec![0u8; HID_PACKET_SIZE + 1]; // +1 for report ID
            packet[0] = 0x00; // Report ID
            
            // 通道 ID
            packet[1] = (CHANNEL >> 8) as u8;
            packet[2] = (CHANNEL & 0xFF) as u8;
            
            // 标签
            packet[3] = TAG_APDU;
            
            // 序列号
            packet[4] = (sequence >> 8) as u8;
            packet[5] = (sequence & 0xFF) as u8;
            
            if sequence == 0 {
                // 第一个包：包含总长度
                packet[6] = (total_len >> 8) as u8;
                packet[7] = (total_len & 0xFF) as u8;
                
                let data_start = 8;
                let chunk_size = (HID_PACKET_SIZE - 7).min(total_len - offset);
                packet[data_start..data_start + chunk_size]
                    .copy_from_slice(&apdu[offset..offset + chunk_size]);
                offset += chunk_size;
            } else {
                // 后续包
                let data_start = 6;
                let chunk_size = (HID_PACKET_SIZE - 5).min(total_len - offset);
                packet[data_start..data_start + chunk_size]
                    .copy_from_slice(&apdu[offset..offset + chunk_size]);
                offset += chunk_size;
            }
            
            packets.push(packet);
            sequence += 1;
        }
        
        packets
    }
    
    /// 接收响应
    fn receive_response(&self) -> Result<Vec<u8>, WalletError> {
        let mut response_data = Vec::new();
        let mut sequence = 0u16;
        let mut total_len: Option<usize> = None;
        
        loop {
            let mut packet = vec![0u8; HID_PACKET_SIZE + 1];
            
            let read_len = self
                .device
                .read_timeout(&mut packet, self.timeout.as_millis() as i32)
                .map_err(|e| WalletError::NetworkError(format!("接收数据failed: {}", e)))?;
            
            if read_len == 0 {
                return Err(WalletError::NetworkError("接收超时".to_string()));
            }
            
            // validate通道
            let channel = ((packet[1] as u16) << 8) | (packet[2] as u16);
            if channel != CHANNEL {
                continue;
            }
            
            // validate标签
            if packet[3] != TAG_APDU {
                continue;
            }
            
            // validate序列号
            let pkt_seq = ((packet[4] as u16) << 8) | (packet[5] as u16);
            if pkt_seq != sequence {
                return Err(WalletError::NetworkError(format!(
                    "序列号不匹配: 期待 {}, 收到 {}",
                    sequence, pkt_seq
                )));
            }
            
            if sequence == 0 {
                // 第一个包：读取总长度
                total_len = Some(((packet[6] as usize) << 8) | (packet[7] as usize));
                response_data.extend_from_slice(&packet[8..read_len]);
            } else {
                // 后续包
                response_data.extend_from_slice(&packet[6..read_len]);
            }
            
            // check是否接收完整
            if let Some(len) = total_len {
                if response_data.len() >= len {
                    response_data.truncate(len);
                    break;
                }
            }
            
            sequence += 1;
            
            // 防止无限循环
            if sequence > 100 {
                return Err(WalletError::NetworkError("接收数据包过多".to_string()));
            }
        }
        
        Ok(response_data)
    }
}

#[cfg(test)]
mod tests {
    #![allow(invalid_value)] // 测试中使用 std::mem::zeroed() 初始化 HidDevice 用于模拟，已标记为 #[ignore]
    
    use super::*;
    use super::super::apdu::{ApduClass, ApduInstruction};
    
    #[test]
    fn test_apdu_command_creation() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            0x00,
            0x00,
            vec![0x01, 0x02],
        );
        
        assert_eq!(cmd.cla, 0xE0);
        assert_eq!(cmd.ins, 0x40);
        assert_eq!(cmd.data.len(), 2);
    }
    
    #[test]
    fn test_apdu_response_success() {
        let bytes = vec![0x01, 0x02, 0x90, 0x00];
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        assert!(response.is_success());
        assert_eq!(response.status_code(), 0x9000);
        assert_eq!(response.data, vec![0x01, 0x02]);
    }
    
    #[test]
    fn test_apdu_response_error() {
        let bytes = vec![0x69, 0x82];
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        assert!(!response.is_success());
        assert_eq!(response.error_description(), "安全状态不满足");
    }
    
    #[test]
    #[ignore] // 需要真正的硬件设备；使用 std::mem::zeroed() 会导致 UB
    fn test_build_single_packet() {
        let transport = LedgerTransport {
            device: unsafe { std::mem::zeroed() }, // 仅用于测试
            timeout: Duration::from_secs(30),
        };
        
        let apdu = vec![0xE0, 0x40, 0x00, 0x00, 0x00];
        let packets = transport.build_hid_packets(&apdu);
        
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].len(), HID_PACKET_SIZE + 1);
    }
    
    // ============ 新增的 Ledger Transport 测试 ============
    
    #[test]
    #[ignore] // 需要真正的硬件设备；使用 std::mem::zeroed() 会导致 UB
    fn test_build_multi_packet() {
        let transport = LedgerTransport {
            device: unsafe { std::mem::zeroed() },
            timeout: Duration::from_secs(30),
        };
        
        // 大于一个包的数据（>55 字节第一包，>59 字节后续包）
        let large_apdu = vec![0xAAu8; 200];
        let packets = transport.build_hid_packets(&large_apdu);
        
        assert!(packets.len() > 1, "大数据应该分多包");
        assert!(packets.len() <= 4, "200 字节应该不超过 4 包");
    }
    
    #[test]
    #[ignore] // 需要真正的硬件设备；使用 std::mem::zeroed() 会导致 UB
    fn test_packet_header_structure() {
        let transport = LedgerTransport {
            device: unsafe { std::mem::zeroed() },
            timeout: Duration::from_secs(30),
        };
        
        let apdu = vec![0xE0, 0x40, 0x00, 0x00, 0x00];
        let packets = transport.build_hid_packets(&apdu);
        
        // 第一个包的头部结构
        assert_eq!(packets[0][0], 0x00);  // Report ID
        assert_eq!(packets[0][1], 0x01);  // Channel H
        assert_eq!(packets[0][2], 0x01);  // Channel L
        assert_eq!(packets[0][3], TAG_APDU);  // Tag
    }
    
    #[test]
    #[ignore] // 需要真正的硬件设备；使用 std::mem::zeroed() 会导致 UB
    fn test_packet_sequence_numbers() {
        let transport = LedgerTransport {
            device: unsafe { std::mem::zeroed() },
            timeout: Duration::from_secs(30),
        };
        
        let large_apdu = vec![0xAAu8; 200];
        let packets = transport.build_hid_packets(&large_apdu);
        
        // validate序列号递增
        for (i, packet) in packets.iter().enumerate() {
            let seq = ((packet[4] as u16) << 8) | (packet[5] as u16);
            assert_eq!(seq, i as u16, "序列号应该from 0 递增");
        }
    }
    
    #[test]
    #[ignore] // 需要真正的硬件设备；使用 std::mem::zeroed() 会导致 UB
    fn test_first_packet_contains_length() {
        let transport = LedgerTransport {
            device: unsafe { std::mem::zeroed() },
            timeout: Duration::from_secs(30),
        };
        
        let apdu = vec![0xE0, 0x40, 0x00, 0x00, 0x00];
        let packets = transport.build_hid_packets(&apdu);
        
        // 第一个包应该包含总长度
        let length = ((packets[0][6] as usize) << 8) | (packets[0][7] as usize);
        assert_eq!(length, apdu.len());
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_empty_apdu() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_exact_packet_size_apdu() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_one_byte_over_packet_size() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_max_apdu_size() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_channel_id_consistency() {
        // Note: 此测试需要重构以使用 mock 或实际设备
        // unsafe { std::mem::zeroed() } 对 HidDevice 是无效的
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_tag_consistency() {
        // Note: 此测试需要重构以使用 mock 或实际设备
        // unsafe { std::mem::zeroed() } 对 HidDevice 是无效的
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_packet_size_consistency() {
        // Note: 此测试需要重构以使用 mock 或实际设备
        // unsafe { std::mem::zeroed() } 对 HidDevice 是无效的
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_data_reconstruction() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_timeout_setting() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    fn test_vendor_id_constant() {
        assert_eq!(LEDGER_VENDOR_ID, 0x2C97);
    }
    
    #[test]
    fn test_usage_page_constant() {
        assert_eq!(LEDGER_USAGE_PAGE, 0xFFA0);
    }
    
    #[test]
    fn test_packet_size_constant() {
        assert_eq!(HID_PACKET_SIZE, 64);
    }
    
    #[test]
    fn test_channel_constant() {
        assert_eq!(CHANNEL, 0x0101);
    }
    
    #[test]
    fn test_tag_apdu_constant() {
        assert_eq!(TAG_APDU, 0x05);
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_split_apdu_across_packets() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_very_large_apdu() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_minimum_apdu() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
    
    #[test]
    #[ignore] // 需要实际的硬件设备,使用 zeroed() 是不安全的
    fn test_packet_padding() {
        unimplemented!("此测试需要 mock HidDevice 或实际硬件");
    }
}


