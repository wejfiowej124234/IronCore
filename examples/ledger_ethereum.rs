//! Ledger Ethereum 示例
//! 
//! 演示如何使用 Ledger 硬件钱包进行 Ethereum 操作

#[cfg(feature = "ledger")]
use defi_hot_wallet::hardware::ledger::{
    bitcoin_app::Bip32Path,
    ethereum_app::LedgerEthereumApp,
};

#[cfg(feature = "ledger")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🔐 Ledger Ethereum 硬件钱包示例\n");
    println!("═══════════════════════════════════════════════════════\n");
    
    // 1. 连接设备
    println!("📝 步骤 1: 连接 Ledger 设备");
    println!("  请确保：");
    println!("  - Ledger 设备已连接到 USB");
    println!("  - 设备已解锁");
    println!("  - Ethereum App 已打开");
    println!();
    
    match LedgerEthereumApp::connect() {
        Ok(app) => {
            println!("✅ 成功连接到 Ledger 设备！\n");
            
            // 2. 获取以太坊地址
            println!("📝 步骤 2: 获取 Ethereum 地址");
            
            // 标准 BIP44 路径
            let path_str = "m/44'/60'/0'/0/0";
            println!("  使用路径: {} (BIP44 标准)", path_str);
            
            match Bip32Path::from_str(path_str) {
                Ok(path) => {
                    match app.get_address(&path, false) {
                        Ok((pubkey, address)) => {
                            println!("  ✅ 地址获取成功！");
                            println!("    公钥长度: {} 字节", pubkey.len());
                            println!("    以太坊地址: {}\n", address);
                            
                            // 3. 签名消息示例
                            println!("📝 步骤 3: 签名个人消息");
                            let message = b"Hello from Ledger!";
                            println!("  消息: {}", String::from_utf8_lossy(message));
                            
                            match app.sign_personal_message(&path, message) {
                                Ok((v, r, s)) => {
                                    println!("  ✅ 签名成功！");
                                    println!("    v: {}", v);
                                    println!("    r: {}", hex::encode(&r));
                                    println!("    s: {}", hex::encode(&s));
                                }
                                Err(e) => {
                                    println!("  ⚠️ 签名失败: {}", e);
                                    println!("  （这通常需要在 Ledger 设备上确认）");
                                }
                            }
                        }
                        Err(e) => {
                            println!("  ❌ 获取地址失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("  ❌ 路径解析失败: {}", e);
                }
            }
            
            println!("\n═══════════════════════════════════════════════════════");
            println!("🎉 示例完成！");
            println!("\n💡 提示：");
            println!("  - EIP-712 签名需要不同的命令");
            println!("  - 交易签名需要完整的交易数据");
            println!("  - 所有签名操作都需要设备确认");
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            println!("\n🔧 故障排除：");
            println!("  1. 确保 Ledger 设备已连接");
            println!("  2. 确保设备已解锁（输入 PIN）");
            println!("  3. 确保 Ethereum App 已打开");
            println!("  4. 在 Windows 上可能需要安装 Ledger 驱动");
            println!("  5. 尝试重新插拔设备");
        }
    }
    
    Ok(())
}

#[cfg(not(feature = "ledger"))]
fn main() {
    println!("❌ 此示例需要 'ledger' feature");
    println!("运行命令: cargo run --example ledger_ethereum --features ledger");
}


