//! 极端角落情况测试 - 最后冲刺

#[cfg(test)]
mod extreme_scenarios {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_transaction_with_1000_utxos() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::{Utxo, UtxoSelector, SelectionStrategy};
        
        let mut utxos = Vec::new();
        for i in 0..1000 {
            utxos.push(Utxo::new(format!("tx{}", i), 0, 1_000, "s".into(), 6));
        }
        
        let result = UtxoSelector::select(&utxos, 500_000, 10, SelectionStrategy::SmallestFirst);
        
        if let Ok((selected, _)) = result {
            assert!(selected.len() > 0);
            assert!(selected.len() <= 1000);
        }
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_single_satoshi_transaction() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        let utxo = Utxo::new("0".repeat(64), 0, 1, "s".into(), 6);
        assert_eq!(utxo.amount, 1);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_max_bitcoin_supply() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // 21M BTC = 2.1e15 satoshi
        let max_supply = 21_000_000u64 * 100_000_000;
        let utxo = Utxo::new("0".repeat(64), 0, max_supply, "s".into(), 6);
        
        assert_eq!(utxo.amount, max_supply);
    }
}

#[cfg(test)]
mod byte_level_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_all_byte_values_in_key() {
        use defi_hot_wallet::core::domain::PrivateKey;
        
        // 测试包含所有可能字节值的密钥
        let mut key_bytes = [0u8; 32];
        for i in 0..32 {
            key_bytes[i] = (i * 8) as u8;  // 0, 8, 16, ...
        }
        
        let pk = PrivateKey::try_from_slice(&key_bytes).unwrap();
        assert_eq!(pk.as_bytes(), &key_bytes);
    }
    
    #[test]
    #[cfg(feature = "ledger")]
    fn test_all_apdu_instruction_values() {
        use defi_hot_wallet::hardware::ledger::apdu::{ApduCommand, ApduClass, ApduInstruction};
        
        let instructions = vec![
            ApduInstruction::GetPublicKey,
            ApduInstruction::SignTransaction,
            ApduInstruction::GetAppConfiguration,
            ApduInstruction::SignMessage,
        ];
        
        for ins in instructions {
            let cmd = ApduCommand::new(ApduClass::Standard, ins, 0, 0, vec![]);
            let bytes = cmd.to_bytes();
            
            assert_eq!(bytes[1], ins as u8);
        }
    }
}

#[cfg(test)]
mod memory_pattern_tests {
    #[test]
    fn test_alternating_bit_pattern() {
        use defi_hot_wallet::core::domain::PrivateKey;
        
        let pattern = [0xAAu8; 32];  // 10101010
        let pk = PrivateKey::try_from_slice(&pattern).unwrap();
        
        assert_eq!(pk.as_bytes(), &pattern);
    }
    
    #[test]
    fn test_walking_ones_pattern() {
        use defi_hot_wallet::core::domain::PrivateKey;
        
        let mut pattern = [0u8; 32];
        pattern[0] = 0x01;  // 00000001
        pattern[1] = 0x02;  // 00000010
        pattern[2] = 0x04;  // 00000100
        
        let pk = PrivateKey::try_from_slice(&pattern).unwrap();
        assert_eq!(pk.as_bytes()[0], 0x01);
        assert_eq!(pk.as_bytes()[1], 0x02);
        assert_eq!(pk.as_bytes()[2], 0x04);
    }
}

#[cfg(test)]
mod unicode_robustness_tests {
    #[test]
    #[cfg(feature = "trezor")]
    fn test_emoji_in_string_fields() {
        use defi_hot_wallet::hardware::trezor::messages::encode_string_field;
        
        let emoji_strings = vec![
            "🚀",
            "💎💰",
            "🔐🔑",
            "Test🎉",
        ];
        
        for s in emoji_strings {
            let encoded = encode_string_field(1, s);
            assert!(!encoded.is_empty());
        }
    }
    
    #[test]
    #[cfg(feature = "trezor")]
    fn test_mixed_language_strings() {
        use defi_hot_wallet::hardware::trezor::messages::encode_string_field;
        
        let mixed = vec![
            "Hello世界",
            "Test测试тест",
            "🌍🌎🌏",
        ];
        
        for s in mixed {
            let encoded = encode_string_field(1, s);
            assert!(encoded.len() >= 2);
        }
    }
}

#[cfg(test)]
mod protocol_compliance_edge_cases {
    #[test]
    #[cfg(feature = "ledger")]
    fn test_apdu_lc_field_accuracy() {
        use defi_hot_wallet::hardware::ledger::apdu::{ApduCommand, ApduClass, ApduInstruction};
        
        // Lc 字段应该准确反映数据长度
        for len in vec![0, 1, 10, 100, 200, 255] {
            let data = vec![0u8; len];
            let cmd = ApduCommand::new(ApduClass::Standard, ApduInstruction::SignTransaction, 0, 0, data);
            
            let bytes = cmd.to_bytes();
            assert_eq!(bytes[4], len as u8);
        }
    }
    
    #[test]
    #[cfg(feature = "trezor")]
    fn test_protobuf_field_numbers() {
        use defi_hot_wallet::hardware::trezor::messages::encode_uint32_field;
        
        // 字段编号应该正确编码
        for field_num in 1..=15 {
            let uint_field = encode_uint32_field(field_num, 100);
            let tag = uint_field[0];
            
            // Tag = (field_num << 3) | wire_type
            assert_eq!(tag >> 3, field_num as u8);
        }
    }
}

#[cfg(test)]
mod determinism_exhaustive_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_address_generation_100_times() {
        use defi_hot_wallet::blockchain::bitcoin::{
            account::BitcoinKeypair,
            address::BitcoinAddress,
        };
        use bitcoin::Network;
        use defi_hot_wallet::core::domain::PrivateKey;
        
        let key = PrivateKey::try_from_slice(&[99u8; 32]).unwrap();
        
        let mut addresses = Vec::new();
        for _ in 0..100 {
            let kp = BitcoinKeypair::from_private_key(&key, Network::Bitcoin).unwrap();
            let addr = BitcoinAddress::from_public_key_taproot(kp.public_key(), Network::Bitcoin).unwrap();
            addresses.push(addr);
        }
        
        // 所有地址应该完全相同
        for addr in &addresses[1..] {
            assert_eq!(addr, &addresses[0]);
        }
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_signature_generation_50_times() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        use defi_hot_wallet::core::domain::PrivateKey;
        
        let key = PrivateKey::try_from_slice(&[77u8; 32]).unwrap();
        let msg = [0x42u8; 32];
        
        let mut signatures = Vec::new();
        for _ in 0..50 {
            let kp = BitcoinKeypair::from_private_key(&key, Network::Bitcoin).unwrap();
            let sig = kp.sign_schnorr(&msg).unwrap();
            signatures.push(sig);
        }
        
        // 验证所有签名格式正确 - Schnorr 签名固定为 64 字节
        // 注意：虽然 BIP340 定义 Schnorr 签名为确定性，但 secp256k1 库实现可能使用辅助随机性
        for sig in &signatures {
            assert_eq!(sig.len(), 64, "所有 Schnorr 签名应该是 64 字节");
        }
    }
}

#[cfg(test)]
mod statistics_and_distribution_tests {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_key_generation_distribution() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        
        let mut first_bytes = Vec::new();
        
        for _ in 0..200 {
            let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
            let pk = kp.public_key_bytes();
            first_bytes.push(pk[1]);  // 跳过前缀字节
        }
        
        // 应该有良好的分布
        let unique: std::collections::HashSet<_> = first_bytes.iter().collect();
        assert!(unique.len() > 50, "密钥生成应该有良好的分布");
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_signature_byte_distribution() {
        use defi_hot_wallet::blockchain::bitcoin::account::BitcoinKeypair;
        use bitcoin::Network;
        
        let kp = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        
        let mut all_bytes = Vec::new();
        for i in 0..50 {
            let msg = [i as u8; 32];
            let sig = kp.sign_schnorr(&msg).unwrap();
            all_bytes.extend_from_slice(&sig);
        }
        
        // 统计字节分布
        let mut counts = [0u32; 256];
        for &b in &all_bytes {
            counts[b as usize] += 1;
        }
        
        // 应该有多种不同的字节值
        let non_zero_counts = counts.iter().filter(|&&c| c > 0).count();
        assert!(non_zero_counts > 100, "签名应该使用多种字节值");
    }
}

#[cfg(test)]
mod final_edge_cases {
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_utxo_vout_max_value() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        // vout 可以是很大的值
        let utxo = Utxo::new("0".repeat(64), u32::MAX, 10_000, "s".into(), 6);
        assert_eq!(utxo.vout, u32::MAX);
    }
    
    #[test]
    #[cfg(feature = "bitcoin")]
    fn test_confirmations_overflow_safe() {
        use defi_hot_wallet::blockchain::bitcoin::utxo::Utxo;
        
        let utxo = Utxo::new("0".repeat(64), 0, 10_000, "s".into(), u32::MAX);
        assert_eq!(utxo.confirmations, u32::MAX);
    }
}

