//! APDU (Application Protocol Data Unit) 命令处理
//! 
//! 实现与 Ledger 设备通信的 APDU 协议

use crate::core::errors::WalletError;
use tracing::debug;

/// APDU 命令类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ApduClass {
    /// 标准 CLA (所有 Ledger apps 都使用 0xE0)
    Standard = 0xE0,
}

/// APDU 指令
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ApduInstruction {
    /// fetch公钥
    GetPublicKey = 0x40,
    /// Sign transaction
    SignTransaction = 0x44,
    /// fetch应用配置
    GetAppConfiguration = 0x06,
    /// sign消息
    SignMessage = 0x48,
}

/// APDU 命令
#[derive(Debug, Clone)]
pub struct ApduCommand {
    /// 类字节
    pub cla: u8,
    /// 指令字节
    pub ins: u8,
    /// 参数 1
    pub p1: u8,
    /// 参数 2
    pub p2: u8,
    /// 数据
    pub data: Vec<u8>,
}

impl ApduCommand {
    /// 创建新的 APDU 命令
    pub fn new(cla: ApduClass, ins: ApduInstruction, p1: u8, p2: u8, data: Vec<u8>) -> Self {
        Self {
            cla: cla as u8,
            ins: ins as u8,
            p1,
            p2,
            data,
        }
    }
    
    /// 序列化为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.cla);
        bytes.push(self.ins);
        bytes.push(self.p1);
        bytes.push(self.p2);
        
        if !self.data.is_empty() {
            bytes.push(self.data.len() as u8);
            bytes.extend_from_slice(&self.data);
        } else {
            bytes.push(0);
        }
        
        debug!("APDU 命令: CLA={:02X} INS={:02X} P1={:02X} P2={:02X} Len={}", 
               self.cla, self.ins, self.p1, self.p2, self.data.len());
        
        bytes
    }
}

/// APDU 响应
#[derive(Debug, Clone)]
pub struct ApduResponse {
    /// 数据部分
    pub data: Vec<u8>,
    /// 状态字 1
    pub sw1: u8,
    /// 状态字 2
    pub sw2: u8,
}

impl ApduResponse {
    /// from字节数组解析
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalletError> {
        if bytes.len() < 2 {
            return Err(WalletError::CryptoError(
                "APDU 响应太短".to_string()
            ));
        }
        
        let len = bytes.len();
        let sw1 = bytes[len - 2];
        let sw2 = bytes[len - 1];
        let data = bytes[..len - 2].to_vec();
        
        debug!("APDU 响应: SW1={:02X} SW2={:02X} DataLen={}", sw1, sw2, data.len());
        
        Ok(Self { data, sw1, sw2 })
    }
    
    /// check是否success
    pub fn is_success(&self) -> bool {
        self.sw1 == 0x90 && self.sw2 == 0x00
    }
    
    /// fetch状态码
    pub fn status_code(&self) -> u16 {
        ((self.sw1 as u16) << 8) | (self.sw2 as u16)
    }
    
    /// fetcherror描述
    pub fn error_description(&self) -> String {
        match (self.sw1, self.sw2) {
            (0x90, 0x00) => "success".to_string(),
            (0x69, 0x82) => "安全状态不满足".to_string(),
            (0x69, 0x85) => "使用条件不满足".to_string(),
            (0x6A, 0x80) => "数据字段error".to_string(),
            (0x6A, 0x82) => "文件未找到".to_string(),
            (0x6D, 0x00) => "指令不支持".to_string(),
            (0x6E, 0x00) => "类不支持".to_string(),
            (0x6F, 0x00) => "Unknown error".to_string(),
            (0x67, 0x00) => "数据长度error".to_string(),
            (0x6B, 0x00) => "参数error".to_string(),
            _ => format!("未知状态: {:04X}", self.status_code()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_apdu_command_serialization() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            0x00,
            0x00,
            vec![0x01, 0x02, 0x03],
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[0], 0xE0); // CLA
        assert_eq!(bytes[1], 0x40); // INS
        assert_eq!(bytes[2], 0x00); // P1
        assert_eq!(bytes[3], 0x00); // P2
        assert_eq!(bytes[4], 0x03); // Lc
        assert_eq!(&bytes[5..], &[0x01, 0x02, 0x03]);
    }
    
    #[test]
    fn test_apdu_response_parsing() {
        let bytes = vec![0x01, 0x02, 0x03, 0x90, 0x00];
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        assert_eq!(response.data, vec![0x01, 0x02, 0x03]);
        assert_eq!(response.sw1, 0x90);
        assert_eq!(response.sw2, 0x00);
        assert!(response.is_success());
    }
    
    #[test]
    fn test_apdu_response_error() {
        let bytes = vec![0x69, 0x82];
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        assert!(!response.is_success());
        assert_eq!(response.status_code(), 0x6982);
        assert_eq!(response.error_description(), "安全状态不满足");
    }
    
    #[test]
    fn test_empty_data_command() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetAppConfiguration,
            0x00,
            0x00,
            vec![],
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes.len(), 5); // CLA + INS + P1 + P2 + Lc(0)
        assert_eq!(bytes[4], 0x00);
    }
    
    // ============ 新增的 APDU 测试 ============
    
    #[test]
    fn test_all_apdu_instructions() {
        // 测试所有指令类型
        let instructions = vec![
            ApduInstruction::GetPublicKey,
            ApduInstruction::SignTransaction,
            ApduInstruction::GetAppConfiguration,
            ApduInstruction::SignMessage,
        ];
        
        for ins in instructions {
            let cmd = ApduCommand::new(ApduClass::Standard, ins, 0x00, 0x00, vec![]);
            let bytes = cmd.to_bytes();
            
            assert_eq!(bytes[0], 0xE0, "CLA 应该是 0xE0");
            assert_eq!(bytes[1], ins as u8, "INS 应该匹配");
        }
    }
    
    #[test]
    fn test_apdu_command_with_max_data() {
        // 最大数据长度（255 字节）
        let max_data = vec![0xAAu8; 255];
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignTransaction,
            0x00,
            0x00,
            max_data.clone(),
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[4], 255); // Lc = 255
        assert_eq!(bytes.len(), 5 + 255);
        assert_eq!(&bytes[5..], &max_data[..]);
    }
    
    #[test]
    fn test_apdu_p1_p2_parameters() {
        // 测试不同的 P1/P2 参数
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            0x01,  // P1
            0x02,  // P2
            vec![],
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[2], 0x01);
        assert_eq!(bytes[3], 0x02);
    }
    
    #[test]
    fn test_apdu_response_only_status() {
        // 只有状态字，无数据
        let bytes = vec![0x90, 0x00];
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        assert!(response.data.is_empty());
        assert!(response.is_success());
    }
    
    #[test]
    fn test_apdu_response_with_data() {
        // 有数据和状态字
        let bytes = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x90, 0x00];
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        assert_eq!(response.data.len(), 5);
        assert_eq!(response.data, vec![0x01, 0x02, 0x03, 0x04, 0x05]);
    }
    
    #[test]
    fn test_all_error_codes() {
        let error_codes = vec![
            (0x69, 0x82, "安全状态不满足"),
            (0x69, 0x85, "使用条件不满足"),
            (0x6A, 0x80, "数据字段error"),
            (0x6A, 0x82, "文件未找到"),
            (0x6D, 0x00, "指令不支持"),
            (0x6E, 0x00, "类不支持"),
            (0x6F, 0x00, "Unknown error"),
            (0x67, 0x00, "数据长度error"),
            (0x6B, 0x00, "参数error"),
        ];
        
        for (sw1, sw2, expected_desc) in error_codes {
            let bytes = vec![sw1, sw2];
            let response = ApduResponse::from_bytes(&bytes).unwrap();
            
            assert!(!response.is_success());
            assert_eq!(response.sw1, sw1);
            assert_eq!(response.sw2, sw2);
            assert_eq!(response.error_description(), expected_desc);
        }
    }
    
    #[test]
    fn test_apdu_status_code_calculation() {
        let response = ApduResponse::from_bytes(&[0x69, 0x82]).unwrap();
        assert_eq!(response.status_code(), 0x6982);
        
        let response2 = ApduResponse::from_bytes(&[0x90, 0x00]).unwrap();
        assert_eq!(response2.status_code(), 0x9000);
    }
    
    #[test]
    fn test_apdu_response_too_short() {
        // 少于 2 字节应该failed
        let result = ApduResponse::from_bytes(&[0x90]);
        assert!(result.is_err());
        
        let result2 = ApduResponse::from_bytes(&[]);
        assert!(result2.is_err());
    }
    
    #[test]
    fn test_apdu_command_cloning() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            0x01,
            0x02,
            vec![0xAA, 0xBB],
        );
        
        let cmd_clone = cmd.clone();
        assert_eq!(cmd.cla, cmd_clone.cla);
        assert_eq!(cmd.ins, cmd_clone.ins);
        assert_eq!(cmd.p1, cmd_clone.p1);
        assert_eq!(cmd.p2, cmd_clone.p2);
        assert_eq!(cmd.data, cmd_clone.data);
    }
    
    #[test]
    fn test_apdu_response_cloning() {
        let response = ApduResponse::from_bytes(&[0x01, 0x02, 0x90, 0x00]).unwrap();
        let response_clone = response.clone();
        
        assert_eq!(response.data, response_clone.data);
        assert_eq!(response.sw1, response_clone.sw1);
        assert_eq!(response.sw2, response_clone.sw2);
    }
    
    #[test]
    fn test_different_instruction_bytes() {
        let cmd1 = ApduCommand::new(ApduClass::Standard, ApduInstruction::GetPublicKey, 0, 0, vec![]);
        let cmd2 = ApduCommand::new(ApduClass::Standard, ApduInstruction::SignTransaction, 0, 0, vec![]);
        
        let bytes1 = cmd1.to_bytes();
        let bytes2 = cmd2.to_bytes();
        
        assert_ne!(bytes1[1], bytes2[1], "不同指令应该有不同的 INS 字节");
    }
    
    #[test]
    fn test_apdu_success_vs_error() {
        let success = ApduResponse::from_bytes(&[0x90, 0x00]).unwrap();
        let error = ApduResponse::from_bytes(&[0x69, 0x82]).unwrap();
        
        assert!(success.is_success());
        assert!(!error.is_success());
        assert_ne!(success.status_code(), error.status_code());
    }
    
    #[test]
    fn test_unknown_error_description() {
        let bytes = vec![0xAB, 0xCD];  // 未知状态码
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        let desc = response.error_description();
        assert!(desc.contains("未知状态") || desc.contains("ABCD"));
    }
    
    #[test]
    fn test_apdu_command_data_sizes() {
        // 测试不同数据大小
        let sizes = vec![0, 1, 10, 50, 100, 200, 255];
        
        for size in sizes {
            let data = vec![0u8; size];
            let cmd = ApduCommand::new(
                ApduClass::Standard,
                ApduInstruction::SignTransaction,
                0, 0,
                data.clone(),
            );
            
            let bytes = cmd.to_bytes();
            assert_eq!(bytes[4], size as u8, "Lc 应该等于数据长度");
            assert_eq!(bytes.len(), 5 + size);
        }
    }
    
    #[test]
    fn test_apdu_response_large_data() {
        // 大数据响应
        let mut data = vec![0xBBu8; 200];
        data.push(0x90);
        data.push(0x00);
        
        let response = ApduResponse::from_bytes(&data).unwrap();
        assert_eq!(response.data.len(), 200);
        assert!(response.is_success());
    }
    
    #[test]
    fn test_sign_message_instruction() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignMessage,
            0x00,
            0x00,
            vec![0x01, 0x02],
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[1], 0x48);  // SignMessage = 0x48
    }
    
    #[test]
    fn test_get_app_configuration_instruction() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetAppConfiguration,
            0x00,
            0x00,
            vec![],
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[1], 0x06);  // GetAppConfiguration = 0x06
    }
    
    #[test]
    fn test_apdu_response_empty_data_with_success() {
        let bytes = vec![0x90, 0x00];
        let response = ApduResponse::from_bytes(&bytes).unwrap();
        
        assert!(response.data.is_empty());
        assert_eq!(response.sw1, 0x90);
        assert_eq!(response.sw2, 0x00);
        assert!(response.is_success());
    }
    
    #[test]
    fn test_apdu_command_serialization_roundtrip() {
        let original_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignTransaction,
            0x01,
            0x02,
            original_data.clone(),
        );
        
        let bytes = cmd.to_bytes();
        
        // validate可以from字节重建参数
        assert_eq!(bytes[0], 0xE0);
        assert_eq!(bytes[1], 0x44);
        assert_eq!(bytes[2], 0x01);
        assert_eq!(bytes[3], 0x02);
        assert_eq!(bytes[4], 5);
        assert_eq!(&bytes[5..], &original_data[..]);
    }
    
    #[test]
    fn test_multiple_error_responses() {
        // 测试多个不同的error响应
        let errors = vec![
            vec![0x69, 0x82],
            vec![0x6D, 0x00],
            vec![0x6A, 0x80],
        ];
        
        for error_bytes in errors {
            let response = ApduResponse::from_bytes(&error_bytes).unwrap();
            assert!(!response.is_success());
            assert!(response.status_code() != 0x9000);
        }
    }
    
    #[test]
    fn test_apdu_response_data_extraction() {
        let data_bytes = vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let mut full_response = data_bytes.clone();
        full_response.extend_from_slice(&[0x90, 0x00]);
        
        let response = ApduResponse::from_bytes(&full_response).unwrap();
        
        assert_eq!(response.data, data_bytes);
        assert!(response.is_success());
    }
    
    #[test]
    fn test_p1_p2_variations() {
        // 测试 P1/P2 的所有组合
        let params = vec![
            (0x00, 0x00),
            (0x01, 0x00),
            (0x00, 0x01),
            (0xFF, 0xFF),
        ];
        
        for (p1, p2) in params {
            let cmd = ApduCommand::new(
                ApduClass::Standard,
                ApduInstruction::GetPublicKey,
                p1, p2,
                vec![],
            );
            
            let bytes = cmd.to_bytes();
            assert_eq!(bytes[2], p1);
            assert_eq!(bytes[3], p2);
        }
    }
    
    #[test]
    fn test_error_description_coverage() {
        // 确保所有error代码都有描述
        let error_codes = vec![
            0x6982, 0x6985, 0x6A80, 0x6A82,
            0x6D00, 0x6E00, 0x6F00, 0x6700, 0x6B00,
        ];
        
        for code in error_codes {
            let sw1 = (code >> 8) as u8;
            let sw2 = (code & 0xFF) as u8;
            let response = ApduResponse::from_bytes(&[sw1, sw2]).unwrap();
            
            let desc = response.error_description();
            assert!(!desc.is_empty());
            assert!(!desc.contains("未知状态") || code > 0x6B00);
        }
    }
    
    #[test]
    fn test_success_code_description() {
        let response = ApduResponse::from_bytes(&[0x90, 0x00]).unwrap();
        assert_eq!(response.error_description(), "success");
    }
    
    #[test]
    fn test_apdu_command_with_single_byte() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            0, 0,
            vec![0x42],
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes[4], 1);
        assert_eq!(bytes[5], 0x42);
    }
    
    #[test]
    fn test_apdu_response_single_data_byte() {
        let response = ApduResponse::from_bytes(&[0xFF, 0x90, 0x00]).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0], 0xFF);
        assert!(response.is_success());
    }
    
    #[test]
    fn test_specific_error_codes() {
        // 安全状态不满足
        let resp1 = ApduResponse::from_bytes(&[0x69, 0x82]).unwrap();
        assert_eq!(resp1.status_code(), 0x6982);
        
        // 指令不支持
        let resp2 = ApduResponse::from_bytes(&[0x6D, 0x00]).unwrap();
        assert_eq!(resp2.status_code(), 0x6D00);
        
        // 参数error
        let resp3 = ApduResponse::from_bytes(&[0x6B, 0x00]).unwrap();
        assert_eq!(resp3.status_code(), 0x6B00);
    }
    
    #[test]
    fn test_apdu_class_consistency() {
        // ApduClass::Standard 应该总是 0xE0
        let cmd1 = ApduCommand::new(ApduClass::Standard, ApduInstruction::GetPublicKey, 0, 0, vec![]);
        let cmd2 = ApduCommand::new(ApduClass::Standard, ApduInstruction::SignTransaction, 0, 0, vec![]);
        
        let bytes1 = cmd1.to_bytes();
        let bytes2 = cmd2.to_bytes();
        
        assert_eq!(bytes1[0], bytes2[0], "相同的 ApduClass 应该有相同的 CLA");
        assert_eq!(bytes1[0], 0xE0);
    }
    
    #[test]
    fn test_apdu_command_data_boundary() {
        // 测试边界数据大小
        let empty_cmd = ApduCommand::new(ApduClass::Standard, ApduInstruction::GetPublicKey, 0, 0, vec![]);
        assert_eq!(empty_cmd.to_bytes().len(), 5);
        
        let one_byte_cmd = ApduCommand::new(ApduClass::Standard, ApduInstruction::GetPublicKey, 0, 0, vec![0]);
        assert_eq!(one_byte_cmd.to_bytes().len(), 6);
        
        let max_cmd = ApduCommand::new(ApduClass::Standard, ApduInstruction::GetPublicKey, 0, 0, vec![0; 255]);
        assert_eq!(max_cmd.to_bytes().len(), 260);
    }
}


