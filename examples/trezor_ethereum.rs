//! Trezor Ethereum 示例
//! 
//! 演示如何使用 Trezor 硬件钱包进行 Ethereum 操作

#[cfg(feature = "trezor")]
use defi_hot_wallet::hardware::{
    ledger::bitcoin_app::Bip32Path,
    trezor::ethereum_app::TrezorEthereumApp,
};

#[cfg(feature = "trezor")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🔐 Trezor Ethereum 硬件钱包示例\n");
    println!("═══════════════════════════════════════════════════════\n");
    
    // 1. 连接设备
    println!("📝 步骤 1: 连接 Trezor 设备");
    println!("  请确保：");
    println!("  - Trezor 设备已连接到 USB");
    println!("  - 设备已解锁（输入 PIN）");
    println!();
    
    match TrezorEthereumApp::connect() {
        Ok(app) => {
            println!("✅ 成功连接到 Trezor 设备！\n");
            
            // 2. 获取以太坊地址
            println!("📝 步骤 2: 获取 Ethereum 地址");
            
            // 标准 BIP44 路径
            let path_str = "m/44'/60'/0'/0/0";
            println!("  使用路径: {} (BIP44 标准)", path_str);
            
            match Bip32Path::from_str(path_str) {
                Ok(path) => {
                    match app.get_address(&path, false) {
                        Ok(address) => {
                            println!("  ✅ 地址获取成功！");
                            println!("    以太坊地址: {}\n", address);
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
            
            // 3. 获取多个地址
            println!("📝 步骤 3: 获取前 3 个地址");
            
            for i in 0..3 {
                let path_str = format!("m/44'/60'/0'/0/{}", i);
                if let Ok(path) = Bip32Path::from_str(&path_str) {
                    if let Ok(address) = app.get_address(&path, false) {
                        println!("  地址 {}: {}", i, address);
                    }
                }
            }
            
            println!("\n═══════════════════════════════════════════════════════");
            println!("🎉 示例完成！");
            println!("\n💡 提示：");
            println!("  - EIP-1559 交易需要完整的交易数据");
            println!("  - 所有签名操作都需要设备确认");
            println!("  - 使用 show_display=true 在设备上验证地址");
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
    println!("运行命令: cargo run --example trezor_ethereum --features trezor");
}

