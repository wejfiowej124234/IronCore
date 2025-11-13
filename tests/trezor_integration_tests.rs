//! Trezor 硬件钱包集成测试
//! 
//! 注意：大部分测试需要实际的 Trezor 设备

#[cfg(feature = "trezor")]
mod trezor_tests {
    use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
    use defi_hot_wallet::hardware::trezor::messages::{
        decode_varint, encode_bip32_path, encode_bool_field, encode_string_field,
        encode_uint32_field, MessageType, TrezorMessage,
    };
    
    #[test]
    fn test_message_serialization() {
        let msg = TrezorMessage::new(MessageType::Initialize, vec![]);
        let serialized = msg.serialize();
        
        assert_eq!(&serialized[0..2], &[0x00, 0x00]); // Type = 0
        assert_eq!(&serialized[2..6], &[0x00, 0x00, 0x00, 0x00]); // Length = 0
    }
    
    #[test]
    fn test_message_deserialization() {
        let data = vec![
            0x00, 0x02, // MessageType::Success
            0x00, 0x00, 0x00, 0x04, // Length = 4
            0x01, 0x02, 0x03, 0x04, // Payload
        ];
        
        let msg = TrezorMessage::deserialize(&data).unwrap();
        assert_eq!(msg.msg_type, MessageType::Success);
        assert_eq!(msg.payload, vec![0x01, 0x02, 0x03, 0x04]);
    }
    
    #[test]
    fn test_varint_encoding_decoding() {
        let test_cases = vec![0u64, 1, 127, 128, 255, 300, 0x8000002C];
        
        for val in test_cases {
            let mut buf = Vec::new();
            super::encode_varint(&mut buf, val);
            let (decoded, len) = decode_varint(&buf).unwrap();
            assert_eq!(decoded, val);
            assert_eq!(len, buf.len());
        }
    }
    
    #[test]
    fn test_bip32_path_encoding() {
        let path = vec![0x8000002C, 0x80000000, 0x80000000, 0, 0];
        let encoded = encode_bip32_path(&path);
        assert!(!encoded.is_empty());
    }
    
    #[test]
    fn test_string_field_encoding() {
        let encoded = encode_string_field(1, "Bitcoin");
        assert!(!encoded.is_empty());
        assert_eq!(encoded[0], 0x0A); // field 1, wire_type 2
    }
    
    #[test]
    fn test_uint32_field_encoding() {
        let encoded = encode_uint32_field(5, 12345);
        assert!(!encoded.is_empty());
        assert_eq!(encoded[0], 0x28); // field 5, wire_type 0
    }
    
    #[test]
    fn test_bool_field_encoding() {
        let encoded_true = encode_bool_field(3, true);
        let encoded_false = encode_bool_field(3, false);
        
        assert_ne!(encoded_true, encoded_false);
    }
    
    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::from_u16(0), Some(MessageType::Initialize));
        assert_eq!(MessageType::from_u16(2), Some(MessageType::Success));
        assert_eq!(MessageType::from_u16(3), Some(MessageType::Failure));
        assert_eq!(MessageType::from_u16(9999), None);
    }
    
    #[test]
    fn test_bip32_path_bitcoin() {
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        assert_eq!(path.path.len(), 5);
        assert_eq!(path.path[0], 0x8000002C); // 44'
        assert_eq!(path.path[1], 0x80000000); // 0'
    }
    
    #[test]
    fn test_bip32_path_ethereum() {
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        assert_eq!(path.path.len(), 5);
        assert_eq!(path.path[1], 0x8000003C); // 60'
    }
    
    #[test]
    fn test_bip32_path_taproot() {
        let path = Bip32Path::from_str("m/86'/0'/0'/0/0").unwrap();
        assert_eq!(path.path[0], 0x80000056); // 86'
    }
    
    #[test]
    fn test_message_roundtrip() {
        let original = TrezorMessage::new(
            MessageType::Ping,
            vec![0xAA, 0xBB, 0xCC, 0xDD],
        );
        
        let serialized = original.serialize();
        let deserialized = TrezorMessage::deserialize(&serialized).unwrap();
        
        assert_eq!(deserialized.msg_type, original.msg_type);
        assert_eq!(deserialized.payload, original.payload);
    }
    
    // 需要实际设备的测试
    
    #[test]
    #[ignore]
    fn test_connect_to_trezor() {
        use defi_hot_wallet::hardware::trezor::device::TrezorDevice;
        
        let result = TrezorDevice::connect();
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore]
    fn test_initialize_device() {
        use defi_hot_wallet::hardware::trezor::device::TrezorDevice;
        
        let mut device = TrezorDevice::connect().unwrap();
        let features = device.initialize().unwrap();
        
        assert!(!features.vendor.is_empty());
        assert!(!features.model.is_empty());
    }
    
    #[test]
    #[ignore]
    fn test_ping_device() {
        use defi_hot_wallet::hardware::trezor::device::TrezorDevice;
        
        let device = TrezorDevice::connect().unwrap();
        let response = device.ping("Hello Trezor").unwrap();
        
        assert_eq!(response, "Pong!");
    }
    
    #[test]
    #[ignore]
    fn test_get_bitcoin_address() {
        use defi_hot_wallet::hardware::trezor::bitcoin_app::TrezorBitcoinApp;
        
        let app = TrezorBitcoinApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        let address = app.get_address(&path, false).unwrap();
        assert!(!address.is_empty());
    }
    
    #[test]
    #[ignore]
    fn test_get_bitcoin_public_key() {
        use defi_hot_wallet::hardware::trezor::bitcoin_app::TrezorBitcoinApp;
        
        let app = TrezorBitcoinApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        let pubkey = app.get_public_key(&path).unwrap();
        assert!(!pubkey.is_empty());
    }
    
    #[test]
    #[ignore]
    fn test_get_ethereum_address() {
        use defi_hot_wallet::hardware::trezor::ethereum_app::TrezorEthereumApp;
        
        let app = TrezorEthereumApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        let address = app.get_address(&path, false).unwrap();
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);
    }
    
    #[test]
    #[ignore]
    fn test_multiple_bitcoin_addresses() {
        use defi_hot_wallet::hardware::trezor::bitcoin_app::TrezorBitcoinApp;
        
        let app = TrezorBitcoinApp::connect().unwrap();
        
        for i in 0..3 {
            let path_str = format!("m/44'/0'/0'/0/{}", i);
            let path = Bip32Path::from_str(&path_str).unwrap();
            
            let address = app.get_address(&path, false).unwrap();
            println!("地址 {}: {}", i, address);
            assert!(!address.is_empty());
        }
    }
}

// 用于测试的辅助函数
#[cfg(feature = "trezor")]
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

#[cfg(not(feature = "trezor"))]
#[test]
fn trezor_feature_disabled() {
    println!("Trezor feature is disabled");
}

