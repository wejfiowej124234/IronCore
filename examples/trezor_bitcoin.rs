//! Trezor Bitcoin 示例
//! 
//! 演示如何使用 Trezor 硬件钱包进行 Bitcoin 操作

#[cfg(feature = "trezor")]
use defi_hot_wallet::hardware::{
    ledger::bitcoin_app::Bip32Path,
    trezor::bitcoin_app::TrezorBitcoinApp,
};

#[cfg(feature = "trezor")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🔐 Trezor Bitcoin 硬件钱包示例\n");
    println!("═══════════════════════════════════════════════════════\n");
    
    // 1. 连接设备
    println!("📝 步骤 1: 连接 Trezor 设备");
    println!("  请确保：");
    println!("  - Trezor 设备已连接到 USB");
    println!("  - 设备已解锁（输入 PIN）");
    println!();
    
    match TrezorBitcoinApp::connect() {
        Ok(app) => {
            println!("✅ 成功连接到 Trezor 设备！\n");
            
            // 2. 获取 Bitcoin 地址
            println!("📝 步骤 2: 获取 Bitcoin 地址");
            
            let paths = vec![
                ("Legacy (BIP44)", "m/44'/0'/0'/0/0"),
                ("SegWit (BIP84)", "m/84'/0'/0'/0/0"),
                ("Taproot (BIP86)", "m/86'/0'/0'/0/0"),
            ];
            
            for (name, path_str) in paths {
                println!("\n  {} 路径: {}", name, path_str);
                
                match Bip32Path::from_str(path_str) {
                    Ok(path) => {
                        match app.get_address(&path, false) {
                            Ok(address) => {
                                println!("    地址: {}", address);
                                println!("    ✅ 成功");
                            }
                            Err(e) => {
                                println!("    ❌ 失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("    ❌ 路径解析失败: {}", e);
                    }
                }
            }
            
            // 3. 获取公钥
            println!("\n📝 步骤 3: 获取扩展公钥");
            let path = Bip32Path::from_str("m/44'/0'/0'/0/0")?;
            
            match app.get_public_key(&path) {
                Ok(pubkey) => {
                    println!("  公钥长度: {} 字节", pubkey.len());
                    println!("  公钥（前16字节）: {}", hex::encode(&pubkey[..16.min(pubkey.len())]));
                    println!("  ✅ 成功");
                }
                Err(e) => {
                    println!("  ❌ 失败: {}", e);
                }
            }
            
            println!("\n═══════════════════════════════════════════════════════");
            println!("🎉 示例完成！");
            println!("\n💡 提示：");
            println!("  - 使用 show_display=true 可在设备上显示地址");
            println!("  - 签名交易需要用户在设备上确认");
            println!("  - 支持 BIP44/84/86 标准路径");
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            println!("\n🔧 故障排除：");
            println!("  1. 确保 Trezor 设备已连接");
            println!("  2. 确保设备已解锁（输入 PIN）");
            println!("  3. Windows: 可能需要安装 Trezor Bridge");
            println!("  4. Linux: 需要配置 udev 规则");
            println!("  5. 尝试重新插拔设备");
        }
    }
    
    Ok(())
}

#[cfg(not(feature = "trezor"))]
fn main() {
    println!("❌ 此示例需要 'trezor' feature");
    println!("运行命令: cargo run --example trezor_bitcoin --features trezor");
}

