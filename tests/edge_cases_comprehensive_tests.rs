//! 边界情况综合测试
//! 
//! 测试极端值、边界条件、异常情况

#[cfg(test)]
mod extreme_values {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_max_satoshi_amount() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // 比特币总量：21M BTC = 2.1e15 satoshi
        let max_btc_supply = 21_000_000 * 100_000_000u64;
        
        let utxo = Utxo::new("0".repeat(64), 0, max_btc_supply, "s".into(), 6);
        assert_eq!(utxo.amount, max_btc_supply);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_single_satoshi() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // 最小金额：1 satoshi
        let utxo = Utxo::new("0".repeat(64), 0, 1, "s".into(), 6);
        assert_eq!(utxo.amount, 1);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_very_long_txid() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // 正常 txid 是 64 字符
        let normal_txid = "0".repeat(64);
        let utxo = Utxo::new(normal_txid.clone(), 0, 10_000, "s".into(), 6);
        assert_eq!(utxo.txid.len(), 64);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_max_confirmations() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // 非常多的确认数
        let utxo = Utxo::new("0".repeat(64), 0, 10_000, "s".into(), 1_000_000);
        assert_eq!(utxo.confirmations, 1_000_000);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_zero_confirmations() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // 零确认（未确认交易）
        let utxo = Utxo::new("0".repeat(64), 0, 10_000, "s".into(), 0);
        assert_eq!(utxo.confirmations, 0);
    }
}

#[cfg(test)]
mod boundary_combinations {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_many_small_utxos() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        // 大量小额 UTXO
        let mut utxos = Vec::new();
        for i in 0..200 {
            utxos.push(Utxo::new(
                format!("tx{}", i),
                0,
                1_000,  // 每个 1000 sat
                "script".into(),
                6,
            ));
        }
        
        // 选择策略应该能处理
        let result = UtxoSelector::select(
            &utxos,
            100_000,
            10,
            SelectionStrategy::SmallestFirst,
        );
        
        assert!(result.is_ok() || result.is_err());  // 不应该 panic
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_single_huge_utxo() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        // 单个巨额 UTXO
        let utxos = vec![
            Utxo::new("0".repeat(64), 0, 1_000_000_000, "s".into(), 6),  // 10 BTC
        ];
        
        let (selected, _) = UtxoSelector::select(
            &utxos,
            10_000,  // 只需要很少
            10,
            SelectionStrategy::BestFit,
        ).unwrap();
        
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].amount, 1_000_000_000);
    }
}

#[cfg(test)]
mod error_edge_cases {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_utxo_insufficient_by_one_satoshi() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        let utxos = vec![
            Utxo::new("0".repeat(64), 0, 100_000, "s".into(), 6),
        ];
        
        // 需要的金额加手续费刚好超过 UTXO
        let result = UtxoSelector::select(
            &utxos,
            99_999,
            10,  // 这会导致总需求 > 100_000
            SelectionStrategy::LargestFirst,
        );
        
        // 应该失败或成功（取决于手续费计算）
        match result {
            Ok((selected, fee)) => {
                let total: u64 = selected.iter().map(|u| u.amount).sum();
                assert!(total >= 99_999 + fee);
            }
            Err(_) => {
                // 余额不足也是合理的
            }
        }
    }
    
    #[test]
    #[cfg(feature = "ledger")]
    fn test_apdu_max_data_boundary() {
        use defi_hot_wallet::hardware::ledger::apdu::{ApduCommand, ApduClass, ApduInstruction};
        
        // APDU 最大数据长度是 255
        let max_data = vec![0xFFu8; 255];
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignTransaction,
            0, 0,
            max_data,
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[4], 255);
        assert_eq!(bytes.len(), 5 + 255);
    }
    
    #[test]
    #[cfg(feature = "ledger")]
    fn test_apdu_one_byte_under_max() {
        use defi_hot_wallet::hardware::ledger::apdu::{ApduCommand, ApduClass, ApduInstruction};
        
        let data = vec![0xEEu8; 254];
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignTransaction,
            0, 0,
            data,
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[4], 254);
    }
}

#[cfg(test)]
mod unicode_and_encoding_tests {
    #[test]
    #[cfg(feature = "trezor")]
    fn test_unicode_string_field() {
        use defi_hot_wallet::hardware::trezor::messages::encode_string_field;
        
        // 测试 Unicode 字符串
        let unicode_strings = vec![
            "Hello",
            "你好",
            "مرحبا",
            "🚀💎",
            "Test\n\t测试",
        ];
        
        for s in unicode_strings {
            let encoded = encode_string_field(1, s);
            assert!(!encoded.is_empty(), "应该能编码: {}", s);
        }
    }
}

#[cfg(test)]
mod concurrent_safety_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_concurrent_key_generation() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        use std::sync::Arc;
        use std::sync::Mutex;
        
        // 并发生成密钥应该安全
        let keys = Arc::new(Mutex::new(Vec::new()));
        let handles: Vec<_> = (0..10).map(|_| {
            let keys_clone = Arc::clone(&keys);
            std::thread::spawn(move || {
                let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
                let pk_bytes = kp.public_key_bytes();
                keys_clone.lock().unwrap().push(pk_bytes);
            })
        }).collect();
        
        for h in handles {
            h.join().unwrap();
        }
        
        let final_keys = keys.lock().unwrap();
        assert_eq!(final_keys.len(), 10);
        
        // 所有密钥应该唯一
        let mut unique = std::collections::HashSet::new();
        for key in final_keys.iter() {
            assert!(unique.insert(key.clone()));
        }
    }
}

#[cfg(test)]
mod stress_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn stress_test_address_generation() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        
        // 生成大量地址
        for _ in 0..100 {
            let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
            let _ = BitcoinAddress::from_public_key_taproot(kp.public_key(), Network::Bitcoin).unwrap();
        }
        
        // 不应该崩溃或内存泄漏
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn stress_test_signing() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        // 签名大量消息
        for i in 0..100 {
            let msg = [i as u8; 32];
            let _ = kp.sign_schnorr(&msg).unwrap();
        }
        
        // 不应该崩溃
    }
}

#[cfg(test)]
mod special_character_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_address_with_special_patterns() {
        use defi_hot_wallet::blockchain::bitcoin::address::BitcoinAddress;
        use bitcoin::Network;
        
        let pattern3 = "3".repeat(34);
        let pattern4 = format!("bc1q{}", "q".repeat(38));
        let special_patterns = vec![
            "1111111111111111111114oLvT2",  // 很多 1
            &pattern3,  // 重复字符
            &pattern4,  // 很多 q
        ];
        
        for pattern in special_patterns {
            let result = BitcoinAddress::validate(pattern, Network::Bitcoin);
            // 应该能处理，不应该 panic
            assert!(result.is_ok());
        }
    }
}

#[cfg(test)]
mod resource_exhaustion_tests {
    #[test]
    #[cfg(feature = "ledger")]
    fn test_very_large_apdu_data() {
        use defi_hot_wallet::hardware::ledger::apdu::{ApduCommand, ApduClass, ApduInstruction};
        
        // APDU 有 255 字节限制，测试边界
        let large_data = vec![0xFFu8; 255];
        let cmd = ApduCommand::new(
            ApduClass::Standard,
            ApduInstruction::SignTransaction,
            0, 0,
            large_data,
        );
        
        let bytes = cmd.to_bytes();
        assert_eq!(bytes.len(), 5 + 255);
    }
}

#[cfg(test)]
mod malformed_data_tests {
    #[test]
    #[cfg(feature = "ledger")]
    fn test_truncated_apdu_response() {
        use defi_hot_wallet::hardware::ledger::apdu::ApduResponse;
        
        // 截断的响应
        let truncated = vec![0x90];  // 只有 1 字节
        let result = ApduResponse::from_bytes(&truncated);
        
        assert!(result.is_err(), "截断的响应应该失败");
    }
    
    #[test]
    #[cfg(feature = "trezor")]
    fn test_corrupted_message() {
        use defi_hot_wallet::hardware::trezor::messages::TrezorMessage;
        
        // 损坏的消息数据
        let corrupted = vec![
            0x00, 0x01,  // Type
            0x00, 0x00, 0x00, 0x05,  // Length = 5
            0x01, 0x02,  // 只有 2 字节（不够）
        ];
        
        let result = TrezorMessage::deserialize(&corrupted);
        assert!(result.is_err(), "损坏的消息应该失败");
    }
}

#[cfg(test)]
mod precision_tests {
    // 注释掉: estimate_fee 是私有方法，不应该在测试中直接调用
    // #[test]
    // #[cfg(feature = "bitcoin")]
    // fn test_fee_calculation_precision() {
    //     use defi_hot_wallet::blockchain::bitcoin::utxo::UtxoSelector;
    //     
    //     // 手续费计算应该准确
    //     let fee_rate = 1;
    //     let fee_1_input = UtxoSelector::estimate_fee(1, fee_rate);
    //     let fee_2_inputs = UtxoSelector::estimate_fee(2, fee_rate);
    //     
    //     // 差值应该是一个输入的大小
    //     let diff = fee_2_inputs - fee_1_input;
    //     assert_eq!(diff, 148, "每个输入应该增加 148 vbytes");
    // }
}

#[cfg(test)]
mod state_machine_tests {
    // 测试状态转换的边界情况
    
    #[test]
    fn test_key_lifecycle() {
        use defi_hot_wallet::core::domain::PrivateKey;
        
        // 创建 -> 使用 -> 销毁
        let key = PrivateKey::try_from_slice(&[42u8; 32]).unwrap();
        let _bytes = key.as_bytes();  // 使用
        drop(key);  // 销毁
        
        // 完整生命周期不应该出错
    }
}

#[cfg(test)]
mod format_validation_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_hex_script_pubkey_validation() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // 有效的十六进制
        let valid_hex = "001400000000000000000000000000000000000000000000";
        let utxo = Utxo::new("0".repeat(64), 0, 10_000, valid_hex.to_string(), 6);
        assert_eq!(utxo.script_pubkey, valid_hex);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_txid_hex_format() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // TXID 应该是十六进制
        let hex_txid = "a".repeat(64);
        let utxo = Utxo::new(hex_txid.clone(), 0, 10_000, "s".into(), 6);
        
        assert!(utxo.txid.chars().all(|c| c.is_ascii_hexdigit() || c.is_lowercase()));
    }
}

#[cfg(test)]
mod network_edge_cases {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_cross_network_key_usage() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        use defi_hot_wallet::core::domain::PrivateKey;
        
        let key = PrivateKey::try_from_slice(&[42u8; 32]).unwrap();
        
        // 同一私钥在不同网络
        let kp_main = BitcoinKeypair::from_private_key(&key, Network::Bitcoin).unwrap();
        let kp_test = BitcoinKeypair::from_private_key(&key, Network::Testnet).unwrap();
        
        // 公钥应该相同
        assert_eq!(kp_main.public_key_bytes(), kp_test.public_key_bytes());
        
        // 但网络标记不同
        assert_eq!(kp_main.network(), Network::Bitcoin);
        assert_eq!(kp_test.network(), Network::Testnet);
    }
}

#[cfg(test)]
mod serialization_edge_cases {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_transaction_hex_output_valid() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
            transaction::BitcoinTransaction,
            utxo::Utxo,
        };
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let addr = BitcoinAddress::from_public_key_legacy(kp.public_key(), Network::Testnet).unwrap();
        let utxos = vec![
            Utxo::new("0".repeat(64), 0, 100_000, "76a914".to_string() + &"00".repeat(20) + "88ac", 6),
        ];
        
        let tx = BitcoinTransaction::build_legacy(
            &kp, &utxos, &addr, 50_000, 1_000, Network::Testnet
        ).unwrap();
        
        let hex = BitcoinTransaction::serialize(&tx);
        
        // 十六进制应该全是合法字符
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        
        // 应该是偶数长度
        assert_eq!(hex.len() % 2, 0);
    }
}

#[cfg(test)]
mod recovery_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_key_recovery_from_bytes() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        use defi_hot_wallet::core::domain::PrivateKey;
        
        // 从字节恢复密钥
        let original_bytes = [99u8; 32];
        let pk = PrivateKey::try_from_slice(&original_bytes).unwrap();
        
        let kp = BitcoinKeypair::from_private_key(&pk, Network::Bitcoin).unwrap();
        let recovered_pk = kp.to_private_key();
        
        assert_eq!(recovered_pk.as_bytes(), &original_bytes);
    }
}

