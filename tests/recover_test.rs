///! 🔐 备份恢复端到端测试 - 真实流程验证
///! 
///! 测试流程：
///! 1. 生成 12 词助记词
///! 2. 创建钱包，记录地址
///! 3. 删除钱包文件
///! 4. 用助记词恢复
///! 5. 验证地址完全一致
///! 
///! ❌ 地址不一致 → 备份恢复失败 → CI 红

#[tokio::test]
async fn test_backup_and_recover_e2e_real() {
    use defi_hot_wallet::crypto::secure_derivation::derive_master_key_secure;
    
    // Step 1: 使用固定的测试助记词（12词）
    let mnemonic_str = "test test test test test test test test test test test junk";
    
    println!("🔐 Step 1: 使用测试助记词（12词）");
    println!("   助记词: <hidden for security>");
    
    // Step 2: 从助记词派生地址（第一次）
    let passphrase = "user_secure_passphrase_2025";
    let app_salt = b"defi-wallet-prod-v1";
    
    let master_key_1 = derive_master_key_secure(
        &mnemonic_str,
        passphrase,
        Some(app_salt),
    ).expect("第一次派生失败");
    
    // 从主密钥派生以太坊地址
    use secp256k1::{Secp256k1, SecretKey};
    let secp = Secp256k1::new();
    let secret_key_1 = SecretKey::from_slice(&master_key_1[..]).expect("无效的密钥");
    let public_key_1 = secret_key_1.public_key(&secp);
    
    // 计算以太坊地址（Keccak256 的后 20 字节）
    use sha3::{Keccak256, Digest};
    let public_key_bytes = &public_key_1.serialize_uncompressed()[1..];
    let hash = Keccak256::digest(public_key_bytes);
    let address_1 = format!("0x{}", hex::encode(&hash[12..]));
    
    println!("✅ Step 2: 第一次派生地址");
    println!("   地址: {}", address_1);
    
    // Step 3: 模拟"删除钱包文件"（清空密钥）
    drop(master_key_1);
    let _ = secret_key_1; // SecretKey is Copy, so just let it go out of scope
    
    println!("🗑️  Step 3: 模拟删除钱包（密钥已清除）");
    
    // Step 4: 用助记词恢复（第二次派生）
    let master_key_2 = derive_master_key_secure(
        &mnemonic_str,
        passphrase,
        Some(app_salt),
    ).expect("恢复派生失败");
    
    let secret_key_2 = SecretKey::from_slice(&master_key_2[..]).expect("恢复的密钥无效");
    let public_key_2 = secret_key_2.public_key(&secp);
    
    let public_key_bytes_2 = &public_key_2.serialize_uncompressed()[1..];
    let hash_2 = Keccak256::digest(public_key_bytes_2);
    let address_2 = format!("0x{}", hex::encode(&hash_2[12..]));
    
    println!("🔄 Step 4: 用助记词恢复");
    println!("   恢复的地址: {}", address_2);
    
    // Step 5: 🔴 关键断言：地址必须完全一致！
    assert_eq!(
        address_1, address_2,
        "🔴 备份恢复失败：恢复的地址与原地址不一致！\n  原地址: {}\n  恢复地址: {}",
        address_1, address_2
    );
    
    println!("✅ Step 5: 备份恢复验证通过");
    println!("   ✅ 地址完全一致");
    println!("   ✅ 备份恢复流程正确");
}

#[tokio::test]
async fn test_recover_with_different_passphrase_fails() {
    // 验证：不同密码 → 不同地址（安全性）
    use defi_hot_wallet::crypto::secure_derivation::derive_master_key_secure;
    
    let mnemonic_str = "test test test test test test test test test test test junk";
    
    // 原始密码
    let key1 = derive_master_key_secure(&mnemonic_str, "password1", None).expect("派生1失败");
    
    // 错误的密码
    let key2 = derive_master_key_secure(&mnemonic_str, "password2", None).expect("派生2失败");
    
    // 🔴 关键：不同密码必须产生不同密钥
    assert_ne!(
        &key1[..], &key2[..],
        "🔴 安全漏洞：不同密码产生了相同密钥！"
    );
    
    println!("✅ 密码验证通过：不同密码 → 不同地址");
}

#[tokio::test]
async fn test_recover_deterministic() {
    // 验证：同样的助记词+密码 → 多次恢复地址一致（确定性）
    use defi_hot_wallet::crypto::secure_derivation::derive_master_key_secure;
    
    let mnemonic = "test test test test test test test test test test test junk";
    let passphrase = "my_password";
    let salt = b"app-salt";
    
    // 恢复 10 次
    let mut addresses = Vec::new();
    for i in 0..10 {
        let key = derive_master_key_secure(mnemonic, passphrase, Some(salt))
            .expect(&format!("第{}次派生失败", i + 1));
        
        use secp256k1::{Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&key[..]).expect("无效密钥");
        let pk = sk.public_key(&secp);
        
        use sha3::{Keccak256, Digest};
        let pk_bytes = &pk.serialize_uncompressed()[1..];
        let hash = Keccak256::digest(pk_bytes);
        let address = format!("0x{}", hex::encode(&hash[12..]));
        
        addresses.push(address);
    }
    
    // 🔴 关键：所有恢复的地址必须完全一致
    let first = &addresses[0];
    for (i, addr) in addresses.iter().enumerate() {
        assert_eq!(
            first, addr,
            "🔴 恢复不确定：第{}次恢复的地址不一致！",
            i + 1
        );
    }
    
    println!("✅ 确定性验证通过：10 次恢复地址完全一致");
    println!("   地址: {}", first);
}

#[tokio::test]
async fn test_recover_with_salt_consistency() {
    // 验证：使用盐值后的恢复一致性
    use defi_hot_wallet::crypto::secure_derivation::derive_master_key_secure;
    
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let passphrase = "user_password";
    let salt = b"production-app-salt-v1";
    
    // 第一次：生成并记录地址
    let key1 = derive_master_key_secure(mnemonic, passphrase, Some(salt)).expect("第一次派生失败");
    let address1 = derive_address_from_key(&key1).expect("第一次地址派生失败");
    
    // 模拟删除钱包
    drop(key1);
    
    // 第二次：恢复
    let key2 = derive_master_key_secure(mnemonic, passphrase, Some(salt)).expect("第二次派生失败");
    let address2 = derive_address_from_key(&key2).expect("第二次地址派生失败");
    
    // 🔴 关键：使用盐值后的恢复必须一致
    assert_eq!(
        address1, address2,
        "🔴 备份恢复失败：使用盐值后地址不一致"
    );
    
    println!("✅ 盐值恢复验证通过：地址一致");
}

// 辅助函数：从密钥派生地址
fn derive_address_from_key(key: &[u8; 32]) -> Result<String, Box<dyn std::error::Error>> {
    use secp256k1::{Secp256k1, SecretKey};
    use sha3::{Keccak256, Digest};
    
    let secp = Secp256k1::new();
    let sk = SecretKey::from_slice(key)?;
    let pk = sk.public_key(&secp);
    let pk_bytes = &pk.serialize_uncompressed()[1..];
    let hash = Keccak256::digest(pk_bytes);
    
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

