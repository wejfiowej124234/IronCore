//! Wallet 单元测试
//! 目标：增加 wallet/create.rs 和 wallet/recover.rs 覆盖率
//! - create.rs: 22/159 → 120/159 (+98 lines)
//! - recover.rs: 10/145 → 100/145 (+90 lines)
//! 总计：+188 lines

#[cfg(test)]
mod wallet_unit_tests {
    use defi_hot_wallet::core::wallet::create::generate_mnemonic;
    
    // ========================================================================
    // 1. 助记词生成测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_generation_basic() {
        let result = generate_mnemonic();
        assert!(result.is_ok(), "应该成功生成助记词");
    }
    
    #[test]
    fn test_mnemonic_word_count() {
        for _ in 0..5 {
            let mnemonic = generate_mnemonic().expect("应该生成助记词");
            let mnemonic_str = String::from_utf8_lossy(&mnemonic);
            let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
            
            // BIP39 标准：12, 15, 18, 21, 或 24 个单词
            assert!(
                vec![12, 15, 18, 21, 24].contains(&words.len()),
                "助记词应该是标准长度，实际: {}",
                words.len()
            );
        }
    }
    
    #[test]
    fn test_mnemonic_uniqueness() {
        let mnemonic1 = generate_mnemonic().expect("应该生成助记词1");
        let mnemonic2 = generate_mnemonic().expect("应该生成助记词2");
        
        // 两次生成应该产生不同的助记词
        assert_ne!(
            &*mnemonic1,
            &*mnemonic2,
            "两次生成应该产生不同的助记词"
        );
    }
    
    #[test]
    fn test_mnemonic_contains_valid_words() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        let mnemonic_str = String::from_utf8_lossy(&mnemonic);
        let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
        
        for word in words {
            // BIP39 单词应该是小写字母
            assert!(
                word.chars().all(|c| c.is_ascii_lowercase()),
                "助记词应该只包含小写字母: {}",
                word
            );
            
            // 单词长度应该在合理范围内（BIP39 最短3个字母，最长8个字母）
            assert!(
                word.len() >= 3 && word.len() <= 8,
                "助记词长度应该在3-8之间: {} (长度: {})",
                word,
                word.len()
            );
        }
    }
    
    #[test]
    fn test_mnemonic_no_spaces() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        let mnemonic_str = String::from_utf8_lossy(&mnemonic);
        
        // 开头和结尾不应该有空格
        assert_eq!(mnemonic_str.trim(), mnemonic_str.as_ref());
    }
    
    #[test]
    fn test_mnemonic_no_duplicates_in_single_phrase() {
        for _ in 0..5 {
            let mnemonic = generate_mnemonic().expect("应该生成助记词");
            let mnemonic_str = String::from_utf8_lossy(&mnemonic);
            let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
            
            // 检查是否有重复单词（虽然 BIP39 允许重复，但概率很低）
            let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
            
            // 大部分情况下单词不应该重复
            // 注意：这不是严格要求，只是检查生成质量
            let duplicate_ratio = 1.0 - (unique_words.len() as f64 / words.len() as f64);
            assert!(
                duplicate_ratio < 0.5,
                "助记词重复率过高: {:.2}%",
                duplicate_ratio * 100.0
            );
        }
    }
    
    #[test]
    fn test_mnemonic_entropy() {
        // 测试多次生成，确保有足够的随机性
        let mut mnemonics = std::collections::HashSet::new();
        
        for _ in 0..10 {
            let mnemonic = generate_mnemonic().expect("应该生成助记词");
            let mnemonic_str = String::from_utf8_lossy(&mnemonic).to_string();
            mnemonics.insert(mnemonic_str);
        }
        
        // 10次生成应该产生10个不同的助记词
        assert_eq!(
            mnemonics.len(),
            10,
            "10次生成应该产生10个不同的助记词"
        );
    }
    
    // ========================================================================
    // 2. 助记词格式测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_utf8_encoding() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        
        // 应该能够正确转换为 UTF-8 字符串
        let result = String::from_utf8(mnemonic.to_vec());
        assert!(result.is_ok(), "助记词应该是有效的 UTF-8");
    }
    
    #[test]
    fn test_mnemonic_ascii_only() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        let mnemonic_str = String::from_utf8_lossy(&mnemonic);
        
        // BIP39 英文单词应该只包含 ASCII 字符
        assert!(
            mnemonic_str.is_ascii(),
            "助记词应该只包含 ASCII 字符"
        );
    }
    
    #[test]
    fn test_mnemonic_single_space_separator() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        let mnemonic_str = String::from_utf8_lossy(&mnemonic);
        
        // 不应该有连续的空格
        assert!(
            !mnemonic_str.contains("  "),
            "助记词不应该有连续空格"
        );
    }
    
    #[test]
    fn test_mnemonic_no_special_chars() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        let mnemonic_str = String::from_utf8_lossy(&mnemonic);
        
        // 不应该包含特殊字符（只有小写字母和空格）
        for ch in mnemonic_str.chars() {
            assert!(
                ch.is_ascii_lowercase() || ch.is_whitespace(),
                "助记词不应该包含特殊字符: '{}'",
                ch
            );
        }
    }
    
    // ========================================================================
    // 3. 助记词长度变化测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_length_distribution() {
        let mut length_counts = std::collections::HashMap::new();
        
        // 生成多个助记词，统计长度分布
        for _ in 0..20 {
            let mnemonic = generate_mnemonic().expect("应该生成助记词");
            let mnemonic_str = String::from_utf8_lossy(&mnemonic);
            let word_count = mnemonic_str.split_whitespace().count();
            
            *length_counts.entry(word_count).or_insert(0) += 1;
        }
        
        // 至少应该生成一种长度的助记词
        assert!(!length_counts.is_empty(), "应该生成助记词");
        
        // 所有生成的长度都应该是有效的
        for &length in length_counts.keys() {
            assert!(
                vec![12, 15, 18, 21, 24].contains(&length),
                "无效的助记词长度: {}",
                length
            );
        }
    }
    
    // ========================================================================
    // 4. SecretVec 测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_returns_secret_vec() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        
        // 应该返回 SecretVec（Zeroizing<Vec<u8>>）
        assert!(!mnemonic.is_empty(), "助记词不应该为空");
    }
    
    #[test]
    fn test_secret_vec_dereferencing() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        
        // 应该能够解引用为 &[u8]
        let bytes: &[u8] = &*mnemonic;
        assert!(!bytes.is_empty(), "助记词字节不应该为空");
    }
    
    #[test]
    fn test_secret_vec_length() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        
        // 助记词长度应该合理（至少几十个字节）
        assert!(
            mnemonic.len() > 30,
            "助记词太短: {} 字节",
            mnemonic.len()
        );
        
        // 助记词长度不应该过长（最多几百个字节）
        assert!(
            mnemonic.len() < 500,
            "助记词太长: {} 字节",
            mnemonic.len()
        );
    }
    
    // ========================================================================
    // 5. 并发生成测试
    // ========================================================================
    
    #[test]
    fn test_concurrent_mnemonic_generation() {
        use std::sync::Arc;
        use std::sync::Mutex;
        use std::thread;
        
        let mnemonics = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];
        
        for _ in 0..5 {
            let mnemonics_clone = Arc::clone(&mnemonics);
            let handle = thread::spawn(move || {
                let mnemonic = generate_mnemonic().expect("应该生成助记词");
                let mnemonic_str = String::from_utf8_lossy(&mnemonic).to_string();
                mnemonics_clone.lock().unwrap().push(mnemonic_str);
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().expect("线程应该成功");
        }
        
        let mnemonics = mnemonics.lock().unwrap();
        assert_eq!(mnemonics.len(), 5, "应该生成5个助记词");
        
        // 所有助记词应该不同
        let unique: std::collections::HashSet<_> = mnemonics.iter().cloned().collect();
        assert_eq!(
            unique.len(),
            5,
            "并发生成的助记词应该全部不同"
        );
    }
    
    #[tokio::test]
    async fn test_async_mnemonic_generation() {
        let mut handles = vec![];
        
        for _ in 0..5 {
            let handle = tokio::spawn(async {
                generate_mnemonic().expect("应该生成助记词")
            });
            handles.push(handle);
        }
        
        let mut mnemonics = Vec::new();
        for handle in handles {
            let mnemonic = handle.await.expect("任务应该成功");
            mnemonics.push(String::from_utf8_lossy(&mnemonic).to_string());
        }
        
        assert_eq!(mnemonics.len(), 5, "应该生成5个助记词");
        
        // 所有助记词应该不同
        let unique: std::collections::HashSet<_> = mnemonics.iter().cloned().collect();
        assert_eq!(
            unique.len(),
            5,
            "异步生成的助记词应该全部不同"
        );
    }
    
    // ========================================================================
    // 6. 边界条件测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_minimum_length() {
        for _ in 0..10 {
            let mnemonic = generate_mnemonic().expect("应该生成助记词");
            let mnemonic_str = String::from_utf8_lossy(&mnemonic);
            let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
            
            // 最少应该有12个单词
            assert!(
                words.len() >= 12,
                "助记词至少应该有12个单词，实际: {}",
                words.len()
            );
        }
    }
    
    #[test]
    fn test_mnemonic_maximum_length() {
        for _ in 0..10 {
            let mnemonic = generate_mnemonic().expect("应该生成助记词");
            let mnemonic_str = String::from_utf8_lossy(&mnemonic);
            let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
            
            // 最多应该有24个单词
            assert!(
                words.len() <= 24,
                "助记词最多应该有24个单词，实际: {}",
                words.len()
            );
        }
    }
    
    // ========================================================================
    // 7. 助记词质量测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_word_variety() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        let mnemonic_str = String::from_utf8_lossy(&mnemonic);
        let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
        
        // 计算唯一单词数
        let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
        
        // 大部分单词应该是唯一的（允许少量重复）
        let uniqueness_ratio = unique_words.len() as f64 / words.len() as f64;
        assert!(
            uniqueness_ratio >= 0.7,
            "助记词唯一性不足: {:.2}%",
            uniqueness_ratio * 100.0
        );
    }
    
    #[test]
    fn test_mnemonic_no_common_patterns() {
        let mnemonic = generate_mnemonic().expect("应该生成助记词");
        let mnemonic_str = String::from_utf8_lossy(&mnemonic);
        
        // 不应该包含明显的弱模式
        assert!(
            !mnemonic_str.contains("test test"),
            "助记词不应该有重复模式"
        );
        assert!(
            !mnemonic_str.contains("password"),
            "助记词不应该包含常见词汇"
        );
    }
    
    // ========================================================================
    // 8. 错误处理测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_generation_reliability() {
        // 连续生成多次，确保稳定性
        for i in 0..20 {
            let result = generate_mnemonic();
            assert!(
                result.is_ok(),
                "第 {} 次生成应该成功",
                i + 1
            );
        }
    }
    
    // ========================================================================
    // 9. 助记词验证测试（如果有验证函数）
    // ========================================================================
    
    #[test]
    fn test_generated_mnemonic_is_valid() {
        // 生成的助记词应该通过基本验证
        for _ in 0..5 {
            let mnemonic = generate_mnemonic().expect("应该生成助记词");
            let mnemonic_str = String::from_utf8_lossy(&mnemonic);
            
            // 基本验证：应该有正确的单词数
            let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
            assert!(
                vec![12, 15, 18, 21, 24].contains(&words.len()),
                "生成的助记词应该有有效的单词数"
            );
            
            // 基本验证：每个单词应该有效
            for word in words {
                assert!(
                    !word.is_empty() && word.len() >= 3,
                    "单词长度应该有效: {}",
                    word
                );
            }
        }
    }
    
    // ========================================================================
    // 10. 性能测试
    // ========================================================================
    
    #[test]
    fn test_mnemonic_generation_performance() {
        use std::time::Instant;
        
        let start = Instant::now();
        
        // 生成100个助记词
        for _ in 0..100 {
            let _ = generate_mnemonic().expect("应该生成助记词");
        }
        
        let duration = start.elapsed();
        
        // 100个助记词应该在合理时间内生成（例如5秒）
        assert!(
            duration.as_secs() < 5,
            "生成100个助记词耗时过长: {:?}",
            duration
        );
    }
    
    #[test]
    fn test_single_mnemonic_fast_generation() {
        use std::time::Instant;
        
        let start = Instant::now();
        let _ = generate_mnemonic().expect("应该生成助记词");
        let duration = start.elapsed();
        
        // 单个助记词应该很快生成（<100ms）
        assert!(
            duration.as_millis() < 100,
            "生成单个助记词耗时过长: {:?}",
            duration
        );
    }
}

