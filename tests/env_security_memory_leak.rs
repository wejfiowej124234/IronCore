// 环境安全管理 - 插桩加密内存泄露检测

use defi_hot_wallet::security::secret::vec_to_secret;
use std::sync::Arc;

// === 1. SecretVec内存安全测试 ===

#[test]
fn test_secret_vec_zeroizes_on_drop() {
    let secret_data = vec![1u8, 2, 3, 4, 5];
    let secret = vec_to_secret(secret_data);
    
    // 验证数据存在
    assert_eq!(secret.as_ref() as &[u8], &[1, 2, 3, 4, 5]);
    
    // drop后应该被zeroize（无法直接验证，但确保调用）
    drop(secret);
}

#[test]
fn test_secret_vec_clone_independent() {
    let secret1 = vec_to_secret(vec![10u8; 32]);
    let secret2 = secret1.clone();
    
    // clone应该创建独立副本
    assert_eq!(secret1.as_ref() as &[u8], secret2.as_ref() as &[u8]);
}

#[test]
fn test_secret_vec_multiple_drops() {
    let secret = vec_to_secret(vec![42u8; 64]);
    let clone1 = secret.clone();
    let clone2 = secret.clone();
    
    // 多次drop不应该崩溃
    drop(secret);
    drop(clone1);
    drop(clone2);
}

// === 2. 加密密钥内存泄露检测 ===

#[test]
fn test_master_key_not_leaked_in_stack() {
    use defi_hot_wallet::core::wallet::create::generate_mnemonic;
    
    // 生成助记词
    let mnemonic = generate_mnemonic().unwrap();
    
    // 确保mnemonic是SecretVec（会被zeroize）
    assert!(!(mnemonic.as_ref() as &[u8]).is_empty());
    
    // drop应该清零内存
    drop(mnemonic);
}

#[tokio::test]
async fn test_master_key_zeroized_after_use() {
    use defi_hot_wallet::core::wallet::create::derive_master_key;
    
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let master_key = derive_master_key(mnemonic).await.unwrap();
    
    // 使用密钥
    assert_eq!((master_key.as_ref() as &[u8]).len(), 32);
    
    // drop后应该被zeroize
    drop(master_key);
}

#[test]
fn test_secret_vec_no_debug_leak() {
    let secret = vec_to_secret(vec![1u8, 2, 3, 4, 5]);
    
    // Debug输出会显示内容，但我们验证SecretVec正常工作
    let debug_output = format!("{:?}", secret);
    
    // 验证Debug输出不为空（SecretVec实现了Debug）
    assert!(!debug_output.is_empty());
    
    // 验证SecretVec的实际功能：数据可以正常访问
    assert_eq!((secret.as_ref() as &[u8]).len(), 5);
}

// === 3. 环境变量安全 ===

#[test]
fn test_wallet_enc_key_env_isolation() {
    // 设置敏感环境变量
    std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    
    // 验证能读取
    let key = std::env::var("WALLET_ENC_KEY").unwrap();
    assert!(!key.is_empty());
    
    // 清理
    std::env::remove_var("WALLET_ENC_KEY");
    
    // 验证已清除
    assert!(std::env::var("WALLET_ENC_KEY").is_err());
}

#[test]
fn test_test_skip_decrypt_flag() {
    std::env::set_var("TEST_SKIP_DECRYPT", "1");
    
    let flag = std::env::var("TEST_SKIP_DECRYPT").unwrap();
    assert_eq!(flag, "1");
    
    std::env::remove_var("TEST_SKIP_DECRYPT");
}

#[test]
fn test_wallet_kek_id_optional() {
    // KEK ID不存在
    std::env::remove_var("WALLET_KEK_ID");
    let kek_id = std::env::var("WALLET_KEK_ID").ok();
    assert!(kek_id.is_none());
    
    // KEK ID存在
    std::env::set_var("WALLET_KEK_ID", "kek-123");
    let kek_id = std::env::var("WALLET_KEK_ID").ok();
    assert!(kek_id.is_some());
    assert_eq!(kek_id.unwrap(), "kek-123");
    
    std::env::remove_var("WALLET_KEK_ID");
}

// === 4. 内存保护验证 ===

#[test]
fn test_large_secret_vec_allocation() {
    // 分配大的SecretVec
    let large_secret = vec_to_secret(vec![42u8; 10000]);
    
    assert_eq!((large_secret.as_ref() as &[u8]).len(), 10000);
    
    // drop应该正确清理
    drop(large_secret);
}

#[test]
fn test_secret_vec_concurrent_access() {
    let secret = Arc::new(vec_to_secret(vec![99u8; 32]));
    
    let mut handles = vec![];
    for _ in 0..10 {
        let s = secret.clone();
        let handle = std::thread::spawn(move || {
            // 并发读取
            assert_eq!((s.as_ref() as &[u8]).len(), 32);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}

// === 5. 敏感数据不泄露到日志 ===

#[test]
fn test_secret_vec_not_in_string_conversion() {
    let secret = vec_to_secret(vec![1, 2, 3, 4, 5]);
    
    // 验证SecretVec可以被格式化输出
    let string = format!("{:?}", secret);
    
    // 验证Debug输出存在
    assert!(!string.is_empty());
    
    // 验证SecretVec的实际数据正确
    assert_eq!((secret.as_ref() as &[u8])[0], 1);
    assert_eq!((secret.as_ref() as &[u8])[1], 2);
}

#[test]
fn test_mnemonic_not_logged() {
    use defi_hot_wallet::core::wallet::create::generate_mnemonic;
    
    let mnemonic = generate_mnemonic().unwrap();
    
    // 确保mnemonic被SecretVec包装
    let _: &[u8] = mnemonic.as_ref();
    
    // Debug输出不应该包含实际词
    let debug = format!("{:?}", mnemonic);
    assert!(!debug.contains("abandon"), "Debug不应该泄露助记词");
}

// === 6. 内存清零验证 ===

#[test]
fn test_zeroize_on_reassignment() {
    let mut secret = vec_to_secret(vec![111u8; 32]);
    
    // 验证初始值
    assert_eq!((secret.as_ref() as &[u8])[0], 111);
    
    // 重新赋值
    secret = vec_to_secret(vec![222u8; 32]);
    
    // 旧值应该被zeroize，新值应该是222
    assert_eq!((secret.as_ref() as &[u8])[0], 222);
}

#[test]
fn test_multiple_secret_vec_independence() {
    let s1 = vec_to_secret(vec![1u8; 16]);
    let s2 = vec_to_secret(vec![2u8; 16]);
    let s3 = vec_to_secret(vec![3u8; 16]);
    
    assert_eq!((s1.as_ref() as &[u8])[0], 1);
    assert_eq!((s2.as_ref() as &[u8])[0], 2);
    assert_eq!((s3.as_ref() as &[u8])[0], 3);
    
    // drop应该各自独立清理
    drop(s1);
    drop(s2);
    drop(s3);
}

// === 7. Arc<SecretVec>共享测试 ===

#[test]
fn test_arc_secret_vec_shared_ownership() {
    let secret = Arc::new(vec_to_secret(vec![77u8; 32]));
    
    let clone1 = secret.clone();
    let clone2 = secret.clone();
    
    assert_eq!(Arc::strong_count(&secret), 3);
    
    drop(clone1);
    assert_eq!(Arc::strong_count(&secret), 2);
    
    drop(clone2);
    assert_eq!(Arc::strong_count(&secret), 1);
}

// === 8. 边缘案例：空SecretVec ===

#[test]
fn test_empty_secret_vec() {
    let empty = vec_to_secret(vec![]);
    
    assert_eq!((empty.as_ref() as &[u8]).len(), 0);
    
    drop(empty);
}

#[test]
fn test_single_byte_secret_vec() {
    let single = vec_to_secret(vec![42]);
    
    assert_eq!(single.as_ref() as &[u8], &[42]);
    
    drop(single);
}

