//! Trezor Protobuf 消息定义
//! 
//! 简化的 Protobuf 消息实现，支持核心操作

use crate::core::errors::WalletError;
use std::io::Write;

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    // 通用消息
    Initialize = 0,
    Ping = 1,
    Success = 2,
    Failure = 3,
    Features = 17,
    
    // 公钥操作
    GetPublicKey = 11,
    PublicKey = 12,
    
    // address操作
    GetAddress = 29,
    Address = 30,
    EthereumGetAddress = 56,
    EthereumAddress = 57,
    
    // sign操作
    SignTx = 15,
    TxRequest = 21,
    TxAck = 22,
    EthereumSignTx = 58,
    EthereumTxRequest = 59,
    EthereumTxAck = 60,
    
    // PIN/Passphrase
    PinMatrixRequest = 18,
    PinMatrixAck = 19,
    PassphraseRequest = 41,
    PassphraseAck = 42,
    
    // 按钮确认
    ButtonRequest = 26,
    ButtonAck = 27,
}

impl MessageType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(Self::Initialize),
            1 => Some(Self::Ping),
            2 => Some(Self::Success),
            3 => Some(Self::Failure),
            17 => Some(Self::Features),
            11 => Some(Self::GetPublicKey),
            12 => Some(Self::PublicKey),
            29 => Some(Self::GetAddress),
            30 => Some(Self::Address),
            56 => Some(Self::EthereumGetAddress),
            57 => Some(Self::EthereumAddress),
            15 => Some(Self::SignTx),
            21 => Some(Self::TxRequest),
            22 => Some(Self::TxAck),
            58 => Some(Self::EthereumSignTx),
            59 => Some(Self::EthereumTxRequest),
            60 => Some(Self::EthereumTxAck),
            18 => Some(Self::PinMatrixRequest),
            19 => Some(Self::PinMatrixAck),
            41 => Some(Self::PassphraseRequest),
            42 => Some(Self::PassphraseAck),
            26 => Some(Self::ButtonRequest),
            27 => Some(Self::ButtonAck),
            _ => None,
        }
    }
}

/// Trezor 消息
#[derive(Debug, Clone)]
pub struct TrezorMessage {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
}

impl TrezorMessage {
    pub fn new(msg_type: MessageType, payload: Vec<u8>) -> Self {
        Self { msg_type, payload }
    }
    
    /// 序列化消息（简化的 Protobuf 格式）
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        
        // 消息类型（2 字节，大端）
        buf.write_all(&(self.msg_type as u16).to_be_bytes()).unwrap();
        
        // 消息长度（4 字节，大端）
        buf.write_all(&(self.payload.len() as u32).to_be_bytes()).unwrap();
        
        // 消息内容
        buf.write_all(&self.payload).unwrap();
        
        buf
    }
    
    /// 反序列化消息
    pub fn deserialize(data: &[u8]) -> Result<Self, WalletError> {
        if data.len() < 6 {
            return Err(WalletError::CryptoError("消息太短".to_string()));
        }
        
        // 读Cancel息类型
        let mut msg_type_bytes = [0u8; 2];
        msg_type_bytes.copy_from_slice(&data[0..2]);
        let msg_type_val = u16::from_be_bytes(msg_type_bytes);
        
        let msg_type = MessageType::from_u16(msg_type_val)
            .ok_or_else(|| WalletError::CryptoError(format!("未知消息类型: {}", msg_type_val)))?;
        
        // 读Cancel息长度
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&data[2..6]);
        let msg_len = u32::from_be_bytes(len_bytes) as usize;
        
        // 读Cancel息内容
        if data.len() < 6 + msg_len {
            return Err(WalletError::CryptoError("消息数据不完整".to_string()));
        }
        
        let payload = data[6..6 + msg_len].to_vec();
        
        Ok(Self { msg_type, payload })
    }
}

/// BIP32 路径编码
pub fn encode_bip32_path(path: &[u32]) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Protobuf 重复字段编码
    for &index in path {
        // Tag: field=1, wire_type=0 (varint)
        buf.push(0x08);
        encode_varint(&mut buf, index as u64);
    }
    
    buf
}

/// Protobuf varint 编码
fn encode_varint(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Protobuf varint 解码
pub fn decode_varint(data: &[u8]) -> Result<(u64, usize), WalletError> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut i = 0;
    
    loop {
        if i >= data.len() {
            return Err(WalletError::CryptoError("Varint 数据不完整".to_string()));
        }
        
        let byte = data[i];
        result |= ((byte & 0x7F) as u64) << shift;
        i += 1;
        
        if byte & 0x80 == 0 {
            break;
        }
        
        shift += 7;
        if shift >= 64 {
            return Err(WalletError::CryptoError("Varint 溢出".to_string()));
        }
    }
    
    Ok((result, i))
}

/// 编码字符串字段
pub fn encode_string_field(field_num: u32, value: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Tag: field_num, wire_type=2 (length-delimited)
    let tag = (field_num << 3) | 2;
    encode_varint(&mut buf, tag as u64);
    
    // 字符串长度
    encode_varint(&mut buf, value.len() as u64);
    
    // 字符串内容
    buf.extend_from_slice(value.as_bytes());
    
    buf
}

/// 编码字节字段
pub fn encode_bytes_field(field_num: u32, value: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Tag
    let tag = (field_num << 3) | 2;
    encode_varint(&mut buf, tag as u64);
    
    // 长度
    encode_varint(&mut buf, value.len() as u64);
    
    // 内容
    buf.extend_from_slice(value);
    
    buf
}

/// 编码 uint32 字段
pub fn encode_uint32_field(field_num: u32, value: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Tag: field_num, wire_type=0 (varint)
    let tag = (field_num << 3) | 0;
    encode_varint(&mut buf, tag as u64);
    
    // Value
    encode_varint(&mut buf, value as u64);
    
    buf
}

/// 编码 bool 字段
pub fn encode_bool_field(field_num: u32, value: bool) -> Vec<u8> {
    encode_uint32_field(field_num, if value { 1 } else { 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_serialization() {
        let msg = TrezorMessage::new(MessageType::Initialize, vec![0x01, 0x02, 0x03]);
        let serialized = msg.serialize();
        
        assert_eq!(&serialized[0..2], &[0x00, 0x00]); // MessageType::Initialize = 0
        assert_eq!(&serialized[2..6], &[0x00, 0x00, 0x00, 0x03]); // length = 3
        assert_eq!(&serialized[6..], &[0x01, 0x02, 0x03]);
    }
    
    #[test]
    fn test_message_deserialization() {
        let data = vec![
            0x00, 0x02, // MessageType::Success = 2
            0x00, 0x00, 0x00, 0x04, // length = 4
            0x0A, 0x0B, 0x0C, 0x0D, // payload
        ];
        
        let msg = TrezorMessage::deserialize(&data).unwrap();
        assert_eq!(msg.msg_type, MessageType::Success);
        assert_eq!(msg.payload, vec![0x0A, 0x0B, 0x0C, 0x0D]);
    }
    
    #[test]
    fn test_varint_encoding() {
        let mut buf = Vec::new();
        encode_varint(&mut buf, 127);
        assert_eq!(buf, vec![0x7F]);
        
        buf.clear();
        encode_varint(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);
        
        buf.clear();
        encode_varint(&mut buf, 300);
        assert_eq!(buf, vec![0xAC, 0x02]);
    }
    
    #[test]
    fn test_varint_decoding() {
        let (val, len) = decode_varint(&[0x7F]).unwrap();
        assert_eq!(val, 127);
        assert_eq!(len, 1);
        
        let (val, len) = decode_varint(&[0x80, 0x01]).unwrap();
        assert_eq!(val, 128);
        assert_eq!(len, 2);
    }
    
    #[test]
    fn test_bip32_path_encoding() {
        let path = vec![0x8000002C, 0x80000000]; // m/44'/0'
        let encoded = encode_bip32_path(&path);
        
        // 应包含两个 varint 字段
        assert!(!encoded.is_empty());
    }
    
    // ============ 新增的 Trezor Messages 测试 ============
    
    #[test]
    fn test_all_message_types() {
        let message_types = vec![
            (MessageType::Initialize, 0),
            (MessageType::Ping, 1),
            (MessageType::Success, 2),
            (MessageType::Failure, 3),
            (MessageType::Features, 17),
            (MessageType::GetPublicKey, 11),
            (MessageType::SignTx, 15),
        ];
        
        for (msg_type, expected_val) in message_types {
            let msg = TrezorMessage::new(msg_type, vec![]);
            let serialized = msg.serialize();
            
            let type_bytes = u16::from_be_bytes([serialized[0], serialized[1]]);
            assert_eq!(type_bytes, expected_val);
        }
    }
    
    #[test]
    fn test_message_type_from_u16() {
        assert_eq!(MessageType::from_u16(0), Some(MessageType::Initialize));
        assert_eq!(MessageType::from_u16(2), Some(MessageType::Success));
        assert_eq!(MessageType::from_u16(9999), None);
        assert_eq!(MessageType::from_u16(11), Some(MessageType::GetPublicKey));
    }
    
    #[test]
    fn test_empty_message() {
        let msg = TrezorMessage::new(MessageType::Initialize, vec![]);
        let serialized = msg.serialize();
        
        assert_eq!(serialized.len(), 6);
        assert_eq!(&serialized[2..6], &[0x00, 0x00, 0x00, 0x00]); // length = 0
    }
    
    #[test]
    fn test_large_message() {
        let payload = vec![0xAAu8; 1000];
        let msg = TrezorMessage::new(MessageType::SignTx, payload.clone());
        let serialized = msg.serialize();
        
        assert_eq!(serialized.len(), 6 + 1000);
        assert_eq!(&serialized[6..], &payload[..]);
    }
    
    #[test]
    fn test_message_roundtrip() {
        let original = TrezorMessage::new(MessageType::Ping, vec![0x01, 0x02, 0x03]);
        let serialized = original.serialize();
        let deserialized = TrezorMessage::deserialize(&serialized).unwrap();
        
        assert_eq!(deserialized.msg_type, original.msg_type);
        assert_eq!(deserialized.payload, original.payload);
    }
    
    #[test]
    fn test_varint_edge_cases() {
        let test_cases = vec![
            0u64,
            1,
            127,      // 单字节最大值
            128,      // 多字节start
            255,
            256,
            16383,    // 双字节最大值
            16384,
            0xFFFFFFFF,
        ];
        
        for val in test_cases {
            let mut buf = Vec::new();
            encode_varint(&mut buf, val);
            let (decoded, _) = decode_varint(&buf).unwrap();
            assert_eq!(decoded, val, "Varint 往返编码failed: {}", val);
        }
    }
    
    #[test]
    fn test_varint_overflow_protection() {
        // 过长的 varint 应该failed
        let bad_varint = vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80];
        let result = decode_varint(&bad_varint);
        assert!(result.is_err(), "过长的 varint 应该failed");
    }
    
    #[test]
    fn test_incomplete_varint() {
        // 不完整的 varint
        let incomplete = vec![0x80];  // 最高位为 1，但没有后续字节
        let result = decode_varint(&incomplete);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_string_field_encoding() {
        let encoded = encode_string_field(1, "Hello");
        
        // Tag: (1 << 3) | 2 = 0x0A
        assert_eq!(encoded[0], 0x0A);
        
        // Length: 5
        assert_eq!(encoded[1], 5);
        
        // Content
        assert_eq!(&encoded[2..], b"Hello");
    }
    
    #[test]
    fn test_empty_string_field() {
        let encoded = encode_string_field(1, "");
        
        assert_eq!(encoded[0], 0x0A);  // Tag
        assert_eq!(encoded[1], 0);     // Length = 0
        assert_eq!(encoded.len(), 2);
    }
    
    #[test]
    fn test_long_string_field() {
        let long_string = "A".repeat(200);
        let encoded = encode_string_field(1, &long_string);
        
        assert!(encoded.len() > 200);
        assert_eq!(encoded[0], 0x0A);  // Tag
    }
    
    #[test]
    fn test_bytes_field_encoding() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let encoded = encode_bytes_field(2, &data);
        
        // Tag: (2 << 3) | 2 = 0x12
        assert_eq!(encoded[0], 0x12);
        
        // Length: 4
        assert_eq!(encoded[1], 4);
        
        // Content
        assert_eq!(&encoded[2..], &data[..]);
    }
    
    #[test]
    fn test_empty_bytes_field() {
        let encoded = encode_bytes_field(3, &[]);
        
        assert_eq!(encoded[0], 0x1A);  // Tag: (3 << 3) | 2
        assert_eq!(encoded[1], 0);     // Length = 0
    }
    
    #[test]
    fn test_uint32_field_encoding() {
        let encoded = encode_uint32_field(5, 12345);
        
        // Tag: (5 << 3) | 0 = 0x28
        assert_eq!(encoded[0], 0x28);
        
        // Value as varint
        assert!(!encoded.is_empty());
    }
    
    #[test]
    fn test_uint32_zero() {
        let encoded = encode_uint32_field(1, 0);
        assert_eq!(encoded[0], 0x08);  // Tag
        assert_eq!(encoded[1], 0);     // Value = 0
    }
    
    #[test]
    fn test_bool_field_true_false() {
        let encoded_true = encode_bool_field(3, true);
        let encoded_false = encode_bool_field(3, false);
        
        assert_ne!(encoded_true, encoded_false);
        
        // True 应该编码为 1
        assert_eq!(encoded_true.last(), Some(&1));
        // False 应该编码为 0
        assert_eq!(encoded_false.last(), Some(&0));
    }
    
    #[test]
    fn test_bip32_path_empty() {
        let path: Vec<u32> = vec![];
        let encoded = encode_bip32_path(&path);
        
        // 空路径应该返回空数组
        assert!(encoded.is_empty());
    }
    
    #[test]
    fn test_bip32_path_single_element() {
        let path = vec![0x8000002C];  // 44'
        let encoded = encode_bip32_path(&path);
        
        assert!(!encoded.is_empty());
    }
    
    #[test]
    fn test_bip32_path_full() {
        let path = vec![0x8000002C, 0x80000000, 0x80000000, 0, 0];  // m/44'/0'/0'/0/0
        let encoded = encode_bip32_path(&path);
        
        assert!(!encoded.is_empty());
    }
    
    #[test]
    fn test_message_deserialization_too_short() {
        let data = vec![0x00];  // 太短
        let result = TrezorMessage::deserialize(&data);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_message_deserialization_incomplete() {
        let data = vec![
            0x00, 0x01,          // Type = 1
            0x00, 0x00, 0x00, 0x10,  // Length = 16
            0x01, 0x02,          // 只有 2 字节数据（期望 16 字节）
        ];
        
        let result = TrezorMessage::deserialize(&data);
        assert!(result.is_err(), "不完整的消息应该failed");
    }
    
    #[test]
    fn test_unknown_message_type() {
        let data = vec![
            0xFF, 0xFF,  // 未知类型
            0x00, 0x00, 0x00, 0x00,
        ];
        
        let result = TrezorMessage::deserialize(&data);
        assert!(result.is_err(), "未知消息类型应该failed");
    }
    
    #[test]
    fn test_field_encoding_different_numbers() {
        // 测试不同字段编号
        let field_1 = encode_uint32_field(1, 100);
        let field_5 = encode_uint32_field(5, 100);
        let field_10 = encode_uint32_field(10, 100);
        
        // 第一个字节应该不同（tag 不同）
        assert_ne!(field_1[0], field_5[0]);
        assert_ne!(field_5[0], field_10[0]);
    }
    
    #[test]
    fn test_large_varint_values() {
        let large_values = vec![
            0x10000u64,
            0x1000000,
            0x100000000,
        ];
        
        for val in large_values {
            let mut buf = Vec::new();
            encode_varint(&mut buf, val);
            let (decoded, _) = decode_varint(&buf).unwrap();
            assert_eq!(decoded, val);
        }
    }
}

