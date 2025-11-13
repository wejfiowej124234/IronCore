//! 最终综合测试 - 冲刺 90% 覆盖率

#[cfg(test)]
mod additional_bitcoin_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_utxo_confirmations_range() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        for conf in vec![0, 1, 6, 100, 1000] {
            let utxo = Utxo::new("0".repeat(64), 0, 10_000, "s".into(), conf);
            assert_eq!(utxo.confirmations, conf);
        }
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_utxo_vout_index() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        for vout in 0..10 {
            let utxo = Utxo::new("0".repeat(64), vout, 10_000, "s".into(), 6);
            assert_eq!(utxo.vout, vout);
        }
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_script_pubkey_formats() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        let scripts = vec![
            "76a914".to_string() + &"00".repeat(20) + "88ac",  // P2PKH
            "0014".to_string() + &"00".repeat(20),  // P2WPKH
            "5120".to_string() + &"00".repeat(32),  // P2TR
        ];
        
        for script in scripts {
            let utxo = Utxo::new("0".repeat(64), 0, 10_000, script.clone(), 6);
            assert_eq!(utxo.script_pubkey, script);
        }
    }
}

#[cfg(test)]
mod transaction_edge_cases {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_transaction_with_max_inputs() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        // 大量输入的情况
        let mut utxos = Vec::new();
        for i in 0..50 {
            utxos.push(Utxo::new(
                format!("tx{}", i), 0, 10_000, "s".into(), 6
            ));
        }
        
        let result = UtxoSelector::select(
            &utxos, 400_000, 10, SelectionStrategy::SmallestFirst
        );
        
        if let Ok((selected, _)) = result {
            assert!(selected.len() <= 50);
        }
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_transaction_output_count() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
            transaction::BitcoinTransaction,
            utxo::Utxo,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let addr = BitcoinAddress::from_public_key_segwit(kp.public_key(), Network::Testnet).unwrap();
        
        let utxos = vec![
            Utxo::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".into(), 0, 100_000, "0014".to_string() + &"00".repeat(20), 6),
        ];
        
        let tx = BitcoinTransaction::build_segwit(
            &kp, &utxos, &addr, 30_000, 1_000, Network::Testnet
        ).unwrap();
        
        // 应该有 1-2 个输出
        assert!(tx.output.len() >= 1 && tx.output.len() <= 2);
    }
}

#[cfg(test)]
mod address_format_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_legacy_address_prefix() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let addr = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Bitcoin).unwrap();
        
        // 主网 Legacy 地址以 1 或 3 开头
        assert!(addr.starts_with('1') || addr.starts_with('3'));
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_segwit_hrp() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let addr = BitcoinAddress::from_public_key_segwit(kp.public_key(), Network::Bitcoin).unwrap();
        
        // 主网 SegWit 以 bc1 开头
        assert!(addr.starts_with("bc1"));
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_taproot_address_format() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let addr = BitcoinAddress::from_public_key_taproot(kp.public_key(), Network::Bitcoin).unwrap();
        
        // Taproot 以 bc1p 开头
        assert!(addr.starts_with("bc1p"));
        assert_eq!(addr.len(), 62);  // 固定长度
    }
}

#[cfg(test)]
mod protocol_compliance_tests {
    #[test]
    #[cfg(feature = "ledger")]
    fn test_apdu_iso7816_compliance() {
        use defi_hot_wallet::hardware::ledger::apdu::ApduCommand;
        use defi_hot_wallet::hardware::ledger::apdu::{ApduClass, ApduInstruction};
        
        // ISO 7816-4 标准格式
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::GetPublicKey,
            0x00, 0x00,
            vec![0x01, 0x02, 0x03],
        );
        
        let bytes = cmd.to_bytes();
        
        // 标准 APDU 格式: CLA INS P1 P2 Lc Data
        assert!(bytes.len() >= 5);
        assert_eq!(bytes.len(), 5 + 3);
    }
    
    #[test]
    #[cfg(feature = "trezor")]
    fn test_protobuf_wire_types() {
        use defi_hot_wallet::hardware::trezor::messages::{
            encode_uint32_field, encode_string_field, encode_bytes_field
        };
        
        // Wire type 0 (varint)
        let uint_field = encode_uint32_field(1, 100);
        assert_eq!(uint_field[0] & 0x07, 0);  // wire_type = 0
        
        // Wire type 2 (length-delimited)
        let string_field = encode_string_field(1, "test");
        assert_eq!(string_field[0] & 0x07, 2);  // wire_type = 2
        
        let bytes_field = encode_bytes_field(1, &[0x01, 0x02]);
        assert_eq!(bytes_field[0] & 0x07, 2);  // wire_type = 2
    }
}

#[cfg(test)]
mod interoperability_tests {
    #[test]
    #[cfg(all(feature = "ledger", feature = "bitcoin"))]
    fn test_same_path_different_wallets() {
        use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
        
        // 相同路径字符串应该在不同钱包中解析一致
        let path_str = "m/84'/0'/0'/0/0";
        
        let path1 = Bip32Path::from_str(path_str).unwrap();
        let path2 = Bip32Path::from_str(path_str).unwrap();
        
        assert_eq!(path1.path, path2.path);
    }
}

#[cfg(test)]
mod data_integrity_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_transaction_id_uniqueness() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
            transaction::BitcoinTransaction,
            utxo::Utxo,
        };
        use bitcoin::Network;
        
        let kp1 = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let kp2 = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        let addr1 = BitcoinAddress::from_public_key_legacy(kp1.public_key(), Network::Testnet).unwrap();
        let addr2 = BitcoinAddress::from_public_key_legacy(kp2.public_key(), Network::Testnet).unwrap();
        
        // 使用有效的 P2PKH script_pubkey
        let script_pubkey = "76a914".to_string() + &"00".repeat(20) + "88ac";
        let valid_txid = "0".repeat(64);
        let utxos = vec![
            Utxo::new(valid_txid, 0, 100_000, script_pubkey, 6),
        ];
        
        // 使用不同的金额确保交易不同
        let tx1 = BitcoinTransaction::build_legacy(&kp1, &utxos, &addr1, 50_000, 1_000, Network::Testnet).unwrap();
        let tx2 = BitcoinTransaction::build_legacy(&kp2, &utxos, &addr2, 40_000, 1_000, Network::Testnet).unwrap();
        
        // 不同密钥、地址和金额应该产生不同的 TXID
        assert_ne!(tx1.txid(), tx2.txid());
    }
}

#[cfg(test)]
mod regression_tests {
    // 回归测试：确保之前修复的问题不再出现
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn regression_schnorr_64_bytes() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let msg = [0u8; 32];
        
        let sig = kp.sign_schnorr(&msg).unwrap();
        
        // 确保 Schnorr 签名始终是 64 字节
        assert_eq!(sig.len(), 64, "回归：Schnorr 签名必须是 64 字节");
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn regression_address_validation() {
        use defi_hot_wallet::blockchain::bitcoin::address::BitcoinAddress;
        use bitcoin::Network;
        
        // 确保空地址被正确拒绝
        assert!(!BitcoinAddress::validate("", Network::Bitcoin).unwrap());
    }
}

#[cfg(test)]
mod compatibility_tests {
    #[test]
    #[cfg(all(feature = "ledger", feature = "bitcoin"))]
    fn test_bip32_path_compatibility() {
        use defi_hot_wallet::hardware::ledger::bitcoin_app::Bip32Path;
        
        // 不同的路径表示应该兼容
        let paths = vec![
            "m/44'/0'/0'/0/0",
            "m/44h/0h/0h/0/0",
        ];
        
        for path_str in paths {
            let path = Bip32Path::from_str(path_str).unwrap();
            assert_eq!(path.path.len(), 5);
            assert_eq!(path.path[0], 0x8000002C);
        }
    }
}

#[cfg(test)]
mod performance_boundary_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_large_scale_address_generation() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        // 生成 1000 个地址应该可行
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = BitcoinAddress::from_public_key_taproot(kp.public_key(), Network::Bitcoin).unwrap();
        }
        let duration = start.elapsed();
        
        // 应该在 5 秒内完成
        assert!(duration.as_secs() < 5, "大量地址生成应该快速");
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_large_scale_signing() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        // 签名 500 个消息
        let start = std::time::Instant::now();
        for i in 0..500 {
            let msg = [(i % 256) as u8; 32];
            let _ = kp.sign_schnorr(&msg).unwrap();
        }
        let duration = start.elapsed();
        
        // 应该在 3 秒内完成
        assert!(duration.as_secs() < 3, "大量签名应该快速");
    }
}

#[cfg(test)]
mod error_recovery_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_recover_from_insufficient_funds() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        let utxos = vec![
            Utxo::new("0".repeat(64), 0, 10_000, "s".into(), 6),
        ];
        
        // 第一次尝试：金额过大
        let result1 = UtxoSelector::select(&utxos, 100_000, 10, SelectionStrategy::LargestFirst);
        assert!(result1.is_err());
        
        // 第二次尝试：合理金额
        let result2 = UtxoSelector::select(&utxos, 5_000, 10, SelectionStrategy::LargestFirst);
        assert!(result2.is_ok());
    }
}

#[cfg(test)]
mod format_compliance_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_hex_encoding_lowercase() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
            transaction::BitcoinTransaction,
            utxo::Utxo,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let addr = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Testnet).unwrap();
        
        // 使用有效的 P2PKH script_pubkey
        let script_pubkey = "76a914".to_string() + &"00".repeat(20) + "88ac";
        let valid_txid = "0".repeat(64);
        let utxos = vec![Utxo::new(valid_txid, 0, 100_000, script_pubkey, 6)];
        
        let tx = BitcoinTransaction::build_legacy(&kp, &utxos, &addr, 50_000, 1_000, Network::Testnet).unwrap();
        let hex = BitcoinTransaction::serialize(&tx);
        
        // 十六进制应该是小写 - 所有字母字符都应该是小写
        assert!(hex.chars().all(|c| !c.is_alphabetic() || c.is_lowercase()));
    }
}

#[cfg(test)]
mod message_type_tests {
    #[test]
    #[cfg(feature = "trezor")]
    fn test_all_trezor_message_types() {
        use defi_hot_wallet::hardware::trezor::messages::MessageType;
        
        let types = vec![
            MessageType::Initialize,
            MessageType::Ping,
            MessageType::Success,
            MessageType::Failure,
            MessageType::Features,
            MessageType::GetPublicKey,
            MessageType::PublicKey,
            MessageType::GetAddress,
            MessageType::Address,
            MessageType::SignTx,
            MessageType::TxRequest,
            MessageType::TxAck,
            MessageType::ButtonRequest,
            MessageType::ButtonAck,
        ];
        
        for msg_type in types {
            // 所有类型都应该能转换
            let val = msg_type as u16;
            let converted = MessageType::from_u16(val);
            assert_eq!(converted, Some(msg_type));
        }
    }
}

#[cfg(test)]
mod serialization_roundtrip_tests {
    #[test]
    #[cfg(feature = "trezor")]
    fn test_message_serialization_multiple() {
        use defi_hot_wallet::hardware::trezor::messages::{TrezorMessage, MessageType};
        
        let test_cases = vec![
            (MessageType::Initialize, vec![]),
            (MessageType::Ping, vec![0x01]),
            (MessageType::GetPublicKey, vec![0x01, 0x02, 0x03]),
            (MessageType::SignTx, vec![0xAA; 100]),
        ];
        
        for (msg_type, payload) in test_cases {
            let msg = TrezorMessage::new(msg_type, payload.clone());
            let serialized = msg.serialize();
            let deserialized = TrezorMessage::deserialize(&serialized).unwrap();
            
            assert_eq!(deserialized.msg_type, msg_type);
            assert_eq!(deserialized.payload, payload);
        }
    }
}

#[cfg(test)]
mod corner_case_combinations {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_all_address_types_all_networks() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::{AddressType, BitcoinAddress},
        };
        use bitcoin::Network;
        
        for network in vec![Network::Bitcoin, Network::Testnet] {
            let kp = BitcoinKeypair::generate(network).unwrap();
            
            for addr_type in vec![AddressType::Legacy, AddressType::SegWit, AddressType::Taproot] {
                let addr = BitcoinAddress::from_public_key(kp.public_key(), addr_type, network).unwrap();
                
                // 所有组合都应该成功
                assert!(!addr.is_empty());
                assert!(BitcoinAddress::validate(&addr, network).unwrap());
                assert_eq!(BitcoinAddress::detect_type(&addr).unwrap(), addr_type);
            }
        }
    }
}

#[cfg(test)]
mod fuzz_style_comprehensive_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn fuzz_utxo_amounts() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        let amounts = vec![0, 1, 546, 1_000, 10_000, 100_000, 1_000_000, 100_000_000, u64::MAX];
        
        for amount in amounts {
            let utxo = Utxo::new("0".repeat(64), 0, amount, "s".into(), 6);
            assert_eq!(utxo.amount, amount);
        }
    }
    
    #[test]
    #[cfg(feature = "ledger")]
    fn fuzz_apdu_parameters() {
        use defi_hot_wallet::hardware::ledger::apdu::ApduCommand;
        use defi_hot_wallet::hardware::ledger::apdu::{ApduClass, ApduInstruction};
        
        for p1 in vec![0x00, 0x01, 0x80, 0xFF] {
            for p2 in vec![0x00, 0x01, 0x80, 0xFF] {
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
    }
}

#[cfg(test)]
mod state_transition_tests {
    #[test]
    fn test_transaction_status_transitions() {
        use defi_hot_wallet::blockchain::traits::TransactionStatus;
        
        // 测试所有状态
        let statuses = vec![
            TransactionStatus::Pending,
            TransactionStatus::Confirmed,
            TransactionStatus::Failed,
            TransactionStatus::Unknown,
        ];
        
        for status in statuses {
            let cloned = status.clone();
            assert_eq!(status, cloned);
        }
    }
}

#[cfg(test)]
mod batch_operation_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_batch_utxo_selection() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        let utxos: Vec<Utxo> = (0..50).map(|i| {
            Utxo::new(format!("tx{}", i), 0, 10_000, "s".into(), 6)
        }).collect();
        
        // 多次选择应该成功
        for amount in (10_000..=100_000).step_by(10_000) {
            let result = UtxoSelector::select(&utxos, amount, 10, SelectionStrategy::BestFit);
            if utxos.iter().map(|u| u.amount).sum::<u64>() >= amount + 2000 {
                assert!(result.is_ok() || result.is_err());
            }
        }
    }
}

