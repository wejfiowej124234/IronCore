//! Bitcoin Taproot 特性演示
//! 
//! 演示 Taproot 的核心特性：
//! 1. Taproot 地址生成
//! 2. Schnorr 签名
//! 3. Taproot 交易构建

#[cfg(feature = "bitcoin")]
use defi_hot_wallet::blockchain::bitcoin::{
    account::BitcoinKeypair,
    address::{AddressType, BitcoinAddress},
    transaction::BitcoinTransaction,
    utxo::Utxo,
};
#[cfg(feature = "bitcoin")]
use bitcoin::Network;

#[cfg(feature = "bitcoin")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 Bitcoin Taproot 特性演示\n");
    println!("═══════════════════════════════════════════════════════\n");
    
    // 1. 密钥对生成
    println!("📝 步骤 1: 生成密钥对");
    let keypair = BitcoinKeypair::generate(Network::Testnet)?;
    println!("✅ 密钥对生成成功");
    println!("  公钥 (压缩):   {} 字节", keypair.public_key_bytes().len());
    println!("  公钥 (未压缩): {} 字节\n", keypair.uncompressed_public_key_bytes().len());
    
    // 2. 比较三种地址类型
    println!("📝 步骤 2: 生成并比较三种地址类型");
    println!("───────────────────────────────────────────────────────");
    
    let legacy = BitcoinAddress::from_public_key(
        keypair.public_key(),
        AddressType::Legacy,
        Network::Testnet,
    )?;
    println!("  Legacy (P2PKH):");
    println!("    地址: {}", legacy);
    println!("    特点: 以 'm' 或 'n' 开头，费用最高");
    println!();
    
    let segwit = BitcoinAddress::from_public_key(
        keypair.public_key(),
        AddressType::SegWit,
        Network::Testnet,
    )?;
    println!("  SegWit (P2WPKH):");
    println!("    地址: {}", segwit);
    println!("    特点: 以 'tb1q' 开头，费用中等");
    println!();
    
    let taproot = BitcoinAddress::from_public_key(
        keypair.public_key(),
        AddressType::Taproot,
        Network::Testnet,
    )?;
    println!("  Taproot (P2TR):");
    println!("    地址: {}", taproot);
    println!("    特点: 以 'tb1p' 开头，费用最低，隐私性最强");
    println!();
    
    // 3. Schnorr 签名演示
    println!("📝 步骤 3: Schnorr 签名 vs ECDSA 签名");
    println!("───────────────────────────────────────────────────────");
    
    let message_hash = [0x42u8; 32];
    
    // ECDSA 签名
    let ecdsa_sig = keypair.sign_ecdsa(&message_hash)?;
    println!("  ECDSA 签名:");
    println!("    长度: {} 字节（可变长度，70-72 字节）", ecdsa_sig.len());
    println!("    用于: Legacy 和 SegWit 交易");
    println!();
    
    // Schnorr 签名
    let schnorr_sig = keypair.sign_schnorr(&message_hash)?;
    println!("  Schnorr 签名:");
    println!("    长度: {} 字节（固定长度）", schnorr_sig.len());
    println!("    用于: Taproot 交易");
    println!("    优势: 更短、更高效、支持聚合");
    println!();
    
    // 4. Taproot 交易构建演示
    println!("📝 步骤 4: 构建 Taproot 交易（演示）");
    println!("───────────────────────────────────────────────────────");
    
    // 创建模拟 UTXO
    let demo_utxo = Utxo::new(
        "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
        0,
        100_000, // 0.001 BTC
        "51200000000000000000000000000000000000000000000000000000000000000000".to_string(), // P2TR script
        6,
    );
    
    println!("  模拟 UTXO:");
    println!("    金额: {} satoshi (0.001 BTC)", demo_utxo.amount);
    println!("    确认数: {}", demo_utxo.confirmations);
    println!();
    
    // 构建交易
    match BitcoinTransaction::build_taproot(
        &keypair,
        &[demo_utxo],
        &taproot,
        50_000, // 0.0005 BTC
        1_000,  // 手续费
        Network::Testnet,
    ) {
        Ok(tx) => {
            println!("✅ Taproot 交易构建成功！");
            println!("  交易 ID: {}", tx.txid());
            println!("  版本: {:?}", tx.version);
            println!("  输入数: {}", tx.input.len());
            println!("  输出数: {}", tx.output.len());
            println!("  Witness 数据: {} 项", tx.input[0].witness.len());
            println!("    → Taproot key-path spend 只需 1 个签名");
            println!();
            
            // 序列化
            let tx_hex = BitcoinTransaction::serialize(&tx);
            println!("  序列化后长度: {} 字节", tx_hex.len() / 2);
            println!("  十六进制 (前 100 字符): {}...", &tx_hex[..100.min(tx_hex.len())]);
        }
        Err(e) => {
            println!("❌ 交易构建失败: {}", e);
        }
    }
    
    println!();
    println!("═══════════════════════════════════════════════════════");
    println!("🎉 Taproot 特性演示完成！");
    println!();
    println!("💡 关键要点:");
    println!("  1. Taproot 使用 Schnorr 签名，更高效且隐私性更强");
    println!("  2. Taproot 地址以 'bc1p' (主网) 或 'tb1p' (测试网) 开头");
    println!("  3. Schnorr 签名固定 64 字节，ECDSA 签名 70-72 字节");
    println!("  4. Taproot 交易费用比 Legacy 低约 30-40%");
    println!("  5. Taproot 支持复杂脚本，但看起来像普通转账（隐私）");
    
    Ok(())
}

#[cfg(not(feature = "bitcoin"))]
fn main() {
    println!("❌ 此示例需要 'bitcoin' feature");
    println!("运行命令: cargo run --example bitcoin_taproot_demo --features bitcoin");
}

