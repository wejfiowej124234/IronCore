//! Bitcoin 转账示例
//! 
//! 演示如何：
//! 1. 生成 Bitcoin 密钥对
//! 2. 生成不同类型的地址（Legacy, SegWit, Taproot）
//! 3. 构建和广播交易

#[cfg(feature = "bitcoin")]
use defi_hot_wallet::blockchain::bitcoin::{
    account::BitcoinKeypair,
    address::{AddressType, BitcoinAddress},
    client::BitcoinClient,
};
#[cfg(feature = "bitcoin")]
use defi_hot_wallet::blockchain::traits::BlockchainClient;
#[cfg(feature = "bitcoin")]
use bitcoin::Network;

#[cfg(feature = "bitcoin")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 Bitcoin 转账示例\n");
    
    // 1. 生成密钥对
    println!("📝 步骤 1: 生成 Bitcoin 密钥对");
    let keypair = BitcoinKeypair::generate(Network::Testnet)?;
    println!("✅ 密钥对生成成功\n");
    
    // 2. 生成不同类型的地址
    println!("📝 步骤 2: 生成地址");
    
    let legacy_address = BitcoinAddress::from_public_key(
        keypair.public_key(),
        AddressType::Legacy,
        Network::Testnet,
    )?;
    println!("  Legacy 地址 (P2PKH):  {}", legacy_address);
    
    let segwit_address = BitcoinAddress::from_public_key(
        keypair.public_key(),
        AddressType::SegWit,
        Network::Testnet,
    )?;
    println!("  SegWit 地址 (P2WPKH): {}", segwit_address);
    
    let taproot_address = BitcoinAddress::from_public_key(
        keypair.public_key(),
        AddressType::Taproot,
        Network::Testnet,
    )?;
    println!("  Taproot 地址 (P2TR):  {}\n", taproot_address);
    
    // 3. 创建客户端（连接到测试网）
    println!("📝 步骤 3: 连接到 Bitcoin 测试网节点");
    let client = BitcoinClient::new(
        "http://localhost:18332".to_string(),
        Network::Testnet,
    )
    .with_auth("bitcoin".to_string(), "password".to_string());
    
    println!("✅ 已连接到: {}\n", client.get_network_name());
    
    // 4. 查询余额
    println!("📝 步骤 4: 查询余额");
    match client.get_balance(&segwit_address).await {
        Ok(balance) => {
            println!("  余额: {} BTC\n", balance);
            
            // 5. 发送交易（如果有余额）
            if balance != "0.00000000" {
                println!("📝 步骤 5: 发送交易");
                println!("  目标地址: tb1q...(请替换为真实地址)");
                println!("  金额: 0.001 BTC");
                
                // 取消注释以实际发送交易
                /*
                let recipient = "tb1q..."; // 替换为真实地址
                let tx_id = client.transfer(
                    &keypair,
                    recipient,
                    100_000, // 0.001 BTC = 100,000 satoshi
                    AddressType::SegWit,
                ).await?;
                
                println!("✅ 交易已发送！");
                println!("  交易 ID: {}\n", tx_id);
                
                // 6. 检查交易状态
                println!("📝 步骤 6: 检查交易状态");
                let status = client.get_transaction_status(&tx_id).await?;
                println!("  状态: {}", status);
                */
                
                println!("⚠️  交易发送代码已注释，取消注释以实际发送");
            } else {
                println!("⚠️  余额为 0，无法发送交易");
                println!("  请向以下地址充值测试币:");
                println!("  {}", segwit_address);
                println!("  测试网水龙头: https://testnet-faucet.com/btc-testnet/");
            }
        }
        Err(e) => {
            println!("❌ 无法查询余额: {}", e);
            println!("  确保 Bitcoin 节点正在运行:");
            println!("  - 测试网节点: http://localhost:18332");
            println!("  - RPC 用户名: bitcoin");
            println!("  - RPC 密码: password");
        }
    }
    
    println!("\n🎉 示例完成！");
    Ok(())
}

#[cfg(not(feature = "bitcoin"))]
fn main() {
    println!("❌ 此示例需要 'bitcoin' feature");
    println!("运行命令: cargo run --example bitcoin_transfer --features bitcoin");
}

