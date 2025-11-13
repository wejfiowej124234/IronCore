//! Ledger Bitcoin 示例
//! 
//! 演示如何使用 Ledger 硬件钱包进行 Bitcoin 操作

#[cfg(feature = "ledger")]
use defi_hot_wallet::hardware::ledger::{
    bitcoin_app::{Bip32Path, LedgerBitcoinApp},
    // device::LedgerDevice, // 未使用
};

#[cfg(feature = "ledger")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🔐 Ledger Bitcoin 硬件钱包示例\n");
    println!("═══════════════════════════════════════════════════════\n");
    
    // 1. 连接设备
    println!("📝 步骤 1: 连接 Ledger 设备");
    println!("  请确保：");
    println!("  - Ledger 设备已连接到 USB");
    println!("  - 设备已解锁");
    println!("  - Bitcoin App 已打开");
    println!();
    
    match LedgerBitcoinApp::connect() {
        Ok(app) => {
            println!("✅ 成功连接到 Ledger 设备！\n");
            
            // 2. 获取应用版本
            println!("📝 步骤 2: 获取 Bitcoin App 版本");
            match app.get_version() {
                Ok(version) => {
                    println!("  Bitcoin App 版本: {}\n", version);
                }
                Err(e) => {
                    println!("  ⚠️ 无法获取版本: {}\n", e);
                }
            }
            
            // 3. 获取公钥和地址
            println!("📝 步骤 3: 获取 Bitcoin 地址");
            
            let paths = vec![
                ("Legacy (BIP44)", "m/44'/0'/0'/0/0"),
                ("SegWit (BIP84)", "m/84'/0'/0'/0/0"),
                ("Taproot (BIP86)", "m/86'/0'/0'/0/0"),
            ];
            
            for (name, path_str) in paths {
                println!("\n  {} 路径: {}", name, path_str);
                
                match Bip32Path::from_str(path_str) {
                    Ok(path) => {
                        match app.get_public_key(&path, false) {
                            Ok((pubkey, address)) => {
                                println!("    公钥长度: {} 字节", pubkey.len());
                                if !address.is_empty() {
                                    println!("    地址: {}", address);
                                }
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
            
            println!("\n═══════════════════════════════════════════════════════");
            println!("🎉 示例完成！");
            println!("\n💡 提示：");
            println!("  - 使用 display=true 可在设备上显示地址");
            println!("  - 签名交易需要用户在设备上确认");
            println!("  - 支持 Legacy、SegWit 和 Taproot 地址");
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            println!("\n🔧 故障排除：");
            println!("  1. 确保 Ledger 设备已连接");
            println!("  2. 确保设备已解锁（输入 PIN）");
            println!("  3. 确保 Bitcoin App 已打开");
            println!("  4. 在 Windows 上可能需要安装 Ledger 驱动");
            println!("  5. 尝试重新插拔设备");
        }
    }
    
    Ok(())
}

#[cfg(not(feature = "ledger"))]
fn main() {
    println!("❌ 此示例需要 'ledger' feature");
    println!("运行命令: cargo run --example ledger_bitcoin --features ledger");
}


