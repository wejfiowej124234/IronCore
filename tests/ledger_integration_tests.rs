//! Ledger 硬件钱包集成测试
//! 
//! 注意：大部分测试需要实际的 Ledger 设备

#[cfg(feature = "ledger")]
mod ledger_tests {
    use defi_hot_wallet::hardware::ledger::{
        apdu::{ApduClass, ApduCommand, ApduInstruction, ApduResponse},
        bitcoin_app::Bip32Path,
    };
    
    #[test]
    fn test_bip32_path_parsing_bitcoin() {
        // BIP44: m/44'/0'/0'/0/0
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        assert_eq!(path.path.len(), 5);
        assert_eq!(path.path[0], 0x8000002C); // 44'
        assert_eq!(path.path[1], 0x80000000); // 0' (Bitcoin)
    }
    
    #[test]
    fn test_bip32_path_parsing_ethereum() {
        // BIP44: m/44'/60'/0'/0/0
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        assert_eq!(path.path.len(), 5);
        assert_eq!(path.path[0], 0x8000002C); // 44'
        assert_eq!(path.path[1], 0x8000003C); // 60' (Ethereum)
    }
    
    #[test]
    fn test_bip32_path_taproot() {
        // BIP86: m/86'/0'/0'/0/0
        let path = Bip32Path::from_str("m/86'/0'/0'/0/0").unwrap();
        
        assert_eq!(path.path[0], 0x80000056); // 86'
    }
    
    #[test]
    fn test_bip32_path_serialization() {
        let path = Bip32Path::new(vec![0x8000002C, 0x80000000, 0x80000000, 0, 0]);
        let bytes = path.to_bytes();
        
        assert_eq!(bytes[0], 5); // 深度
        assert_eq!(bytes.len(), 1 + 5 * 4); // 1 + depth * 4
    }
    
    #[test]
    fn test_apdu_command_get_pubkey() {
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            0x00,
            0x00,
            vec![0x05, 0x80, 0x00, 0x00, 0x2C],
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[0], 0xE0);
        assert_eq!(bytes[1], 0x40);
    }
    
    #[test]
    fn test_apdu_response_success() {
        let response = ApduResponse::from_bytes(&[0x01, 0x02, 0x90, 0x00]).unwrap();
        assert!(response.is_success());
        assert_eq!(response.status_code(), 0x9000);
    }
    
    #[test]
    fn test_apdu_response_errors() {
        let test_cases = vec![
            (vec![0x69, 0x82], "安全状态不满足"),
            (vec![0x69, 0x85], "使用条件不满足"),
            (vec![0x6D, 0x00], "指令不支持"),
        ];
        
        for (bytes, expected) in test_cases {
            let response = ApduResponse::from_bytes(&bytes).unwrap();
            assert!(!response.is_success());
            assert_eq!(response.error_description(), expected);
        }
    }
    
    #[test]
    fn test_bip32_hardened_vs_normal() {
        let path1 = Bip32Path::from_str("m/44'/0'").unwrap();
        let path2 = Bip32Path::from_str("m/44/0").unwrap();
        
        assert!(path1.path[0] >= 0x80000000); // 硬化
        assert!(path2.path[0] < 0x80000000);   // 非硬化
    }
    
    #[test]
    fn test_bip32_path_with_h_notation() {
        let path = Bip32Path::from_str("m/44h/0h/0h/0/0").unwrap();
        
        assert_eq!(path.path[0], 0x8000002C); // 44' 和 44h 相同
    }
    
    #[test]
    fn test_invalid_bip32_path() {
        let result = Bip32Path::from_str("m/invalid/path");
        assert!(result.is_err());
    }
    
    // 以下测试需要实际的 Ledger 设备
    
    #[test]
    #[ignore]
    fn test_connect_to_ledger_device() {
        use defi_hot_wallet::hardware::ledger::device::LedgerDevice;
        
        let result = LedgerDevice::connect();
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore]
    fn test_get_bitcoin_app_configuration() {
        use defi_hot_wallet::hardware::ledger::device::LedgerDevice;
        
        let device = LedgerDevice::connect().unwrap();
        let info = device.get_app_configuration().unwrap();
        
        assert!(!info.name.is_empty());
        assert!(!info.version.is_empty());
    }
    
    #[test]
    #[ignore]
    fn test_get_bitcoin_public_key() {
        use defi_hot_wallet::hardware::ledger::bitcoin_app::LedgerBitcoinApp;
        
        let app = LedgerBitcoinApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/0'/0'/0/0").unwrap();
        
        let (pubkey, address) = app.get_public_key(&path, false).unwrap();
        
        assert!(!pubkey.is_empty());
        assert!(!address.is_empty());
    }
    
    #[test]
    #[ignore]
    fn test_get_ethereum_address() {
        use defi_hot_wallet::hardware::ledger::ethereum_app::LedgerEthereumApp;
        
        let app = LedgerEthereumApp::connect().unwrap();
        let path = Bip32Path::from_str("m/44'/60'/0'/0/0").unwrap();
        
        let (pubkey, address) = app.get_address(&path, false).unwrap();
        
        assert_eq!(pubkey.len(), 65);
        assert!(address.starts_with("0x"));
    }
    
    #[test]
    #[ignore]
    fn test_multiple_addresses() {
        use defi_hot_wallet::hardware::ledger::bitcoin_app::LedgerBitcoinApp;
        
        let app = LedgerBitcoinApp::connect().unwrap();
        
        for i in 0..5 {
            let path_str = format!("m/44'/0'/0'/0/{}", i);
            let path = Bip32Path::from_str(&path_str).unwrap();
            
            let (_pubkey, address) = app.get_public_key(&path, false).unwrap();
            println!("地址 {}: {}", i, address);
        }
    }
}

#[cfg(not(feature = "ledger"))]
#[test]
fn ledger_feature_disabled() {
    println!("Ledger feature is disabled");
}


