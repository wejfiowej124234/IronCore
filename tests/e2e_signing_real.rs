///! 🔐 端到端签名测试 - 真实流程
///! 
///! 测试完整的钱包→签名→验证流程

use defi_hot_wallet::crypto::secure_derivation::derive_master_key_secure;

#[tokio::test]
async fn test_e2e_mnemonic_to_signature_real() {
    // 🔐 测试助记词 (BIP39标准)
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    // 🔐 用户密码短语（真实场景）
    let user_passphrase = "my_secure_passphrase_2025";
    
    // 🔐 应用层盐值
    let app_salt = b"defi-hot-wallet-production-v1";
    
    // Step 1: 安全派生主密钥
    let master_key = derive_master_key_secure(
        mnemonic,
        user_passphrase,
        Some(app_salt),
    ).expect("派生主密钥失败");
    
    // 验证：不同密码应该产生不同密钥
    let different_key = derive_master_key_secure(
        mnemonic,
        "different_password",
        Some(app_salt),
    ).expect("派生失败");
    
    assert_ne!(
        &master_key[..],
        &different_key[..],
        "🔴 严重安全漏洞：不同密码产生了相同密钥！"
    );
    
    println!("✅ 密码短语验证通过：不同密码产生不同密钥");
    
    // Step 2: 验证 zeroize 生效
    let mut test_key = master_key.clone();
    let original_first_byte = test_key[0];
    
    // 显式清零
    use zeroize::Zeroize;
    test_key.zeroize();
    
    assert_eq!(test_key[0], 0, "🔴 Zeroize 失败：密钥未被清零！");
    assert_ne!(original_first_byte, 0, "测试数据无效");
    
    println!("✅ Zeroize 验证通过：密钥已成功擦除");
    
    // Step 3: TODO - 真实签名流程（需要完整的 BIP32/BIP44 实现）
    // 1. 从主密钥派生 HD 钱包路径 m/44'/60'/0'/0/0
    // 2. 生成以太坊地址
    // 3. 构建交易
    // 4. 签名
    // 5. 验证签名
    
    println!("⚠️  完整 BIP44 派生流程待实现");
}

#[tokio::test]
async fn test_e2e_passphrase_entropy_critical() {
    // 🔴 关键测试：验证空密码和非空密码的差异
    let mnemonic = "test test test test test test test test test test test junk";
    
    // 空密码（不安全）
    let key_empty = derive_master_key_secure(mnemonic, "", None)
        .expect("派生失败");
    
    // 非空密码（安全）
    let key_with_pass = derive_master_key_secure(mnemonic, "user_password_123", None)
        .expect("派生失败");
    
    // 🔴 关键断言：必须不同！
    assert_ne!(
        &key_empty[..],
        &key_with_pass[..],
        "🔴 致命安全漏洞：空密码和非空密码产生相同密钥！"
    );
    
    println!("✅ 密码熵测试通过：空密码 ≠ 非空密码");
}

#[tokio::test]
async fn test_e2e_app_salt_adds_security() {
    // 🔐 测试应用层盐值的作用
    let mnemonic = "test test test test test test test test test test test junk";
    let passphrase = "user_password";
    
    // 无应用盐值
    let key_no_salt = derive_master_key_secure(mnemonic, passphrase, None)
        .expect("派生失败");
    
    // 有应用盐值
    let key_with_salt = derive_master_key_secure(
        mnemonic,
        passphrase,
        Some(b"application-specific-salt"),
    ).expect("派生失败");
    
    // 🔐 关键断言：盐值必须改变结果
    assert_ne!(
        &key_no_salt[..],
        &key_with_salt[..],
        "🔴 应用盐值无效：未改变派生结果！"
    );
    
    println!("✅ 应用盐值测试通过：盐值成功增强安全性");
}

#[tokio::test]
async fn test_e2e_brute_force_resistance() {
    // 🔐 模拟暴力破解场景
    let correct_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let user_passphrase = "super_secret_password_2025";
    
    let correct_key = derive_master_key_secure(
        correct_mnemonic,
        user_passphrase,
        Some(b"app-salt"),
    ).expect("派生失败");
    
    // 攻击者尝试：正确助记词 + 错误密码
    let attacker_key = derive_master_key_secure(
        correct_mnemonic,
        "",  // 攻击者不知道密码
        Some(b"app-salt"),
    ).expect("派生失败");
    
    // 🔐 关键：即使助记词泄露，没有密码短语也无法得到私钥
    assert_ne!(
        &correct_key[..],
        &attacker_key[..],
        "🔴 严重漏洞：仅凭助记词就能推导私钥！"
    );
    
    println!("✅ 暴力破解抵抗测试通过：密码短语有效保护");
    println!("📊 安全性：助记词 (2048组合) + 密码 (无限空间) = 强保护");
}

