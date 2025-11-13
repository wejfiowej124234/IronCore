///! 🔐 密码学性能基准测试 - 钱包签名不能慢
///! 
///! 性能要求：
///! - 助记词→私钥：< 10ms
///! - 签名交易：< 10ms
///! - 总流程：< 20ms
///! 
///! 超过阈值 → CI 失败

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use defi_hot_wallet::crypto::secure_derivation::derive_master_key_secure;

/// 🔴 性能基准：助记词 → 私钥派生
/// 要求：< 10ms（用户体验阈值）
fn bench_mnemonic_to_key(c: &mut Criterion) {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let passphrase = "test_password_2025";
    let salt = b"app-salt-v1";
    
    c.bench_function("mnemonic_to_master_key", |b| {
        b.iter(|| {
            derive_master_key_secure(
                black_box(mnemonic),
                black_box(passphrase),
                black_box(Some(salt)),
            ).expect("derive failed")
        });
    });
}

/// 🔴 性能基准：签名交易
/// 要求：< 10ms
fn bench_transaction_signing(c: &mut Criterion) {
    use secp256k1::{Secp256k1, Message, SecretKey};
    use sha2::{Sha256, Digest};
    
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&[1u8; 32]).expect("valid key");
    
    // 模拟交易哈希
    let tx_data = b"transfer 1.0 ETH to 0x1234...";
    let tx_hash = Sha256::digest(tx_data);
    let message = Message::from_slice(&tx_hash).expect("valid message");
    
    c.bench_function("sign_transaction", |b| {
        b.iter(|| {
            secp.sign_ecdsa(
                black_box(&message),
                black_box(&secret_key),
            )
        });
    });
}

/// 🔴 性能基准：完整流程（助记词→签名）
/// 要求：< 20ms（总延迟）
fn bench_full_signing_flow(c: &mut Criterion) {
    use secp256k1::{Secp256k1, Message, SecretKey};
    use sha2::{Sha256, Digest};
    
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let passphrase = "user_pass";
    let salt = b"app-salt";
    
    c.bench_function("full_signing_flow", |b| {
        b.iter(|| {
            // 1. 派生主密钥
            let master_key = derive_master_key_secure(
                black_box(mnemonic),
                black_box(passphrase),
                black_box(Some(salt)),
            ).expect("derive failed");
            
            // 2. 签名
            let secp = Secp256k1::new();
            let secret_key = SecretKey::from_slice(&master_key[..]).expect("valid key");
            let tx_hash = Sha256::digest(b"tx_data");
            let message = Message::from_slice(&tx_hash).expect("valid");
            
            secp.sign_ecdsa(
                black_box(&message),
                black_box(&secret_key),
            )
        });
    });
}

criterion_group!(benches, bench_mnemonic_to_key, bench_transaction_signing, bench_full_signing_flow);
criterion_main!(benches);

