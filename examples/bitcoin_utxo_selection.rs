//! Bitcoin UTXO 选择策略演示
//! 
//! 展示不同的 UTXO 选择策略及其适用场景

#[cfg(feature = "bitcoin")]
use defi_hot_wallet::blockchain::bitcoin::utxo::{SelectionStrategy, Utxo, UtxoSelector};

#[cfg(feature = "bitcoin")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Bitcoin UTXO 选择策略演示\n");
    println!("═══════════════════════════════════════════════════════\n");
    
    // 创建测试 UTXO 集
    let utxos = vec![
        Utxo::new(
            "tx1".to_string(),
            0,
            100_000, // 0.001 BTC
            "script".to_string(),
            10,
        ),
        Utxo::new(
            "tx2".to_string(),
            1,
            50_000, // 0.0005 BTC
            "script".to_string(),
            5,
        ),
        Utxo::new(
            "tx3".to_string(),
            2,
            200_000, // 0.002 BTC
            "script".to_string(),
            20,
        ),
        Utxo::new(
            "tx4".to_string(),
            3,
            30_000, // 0.0003 BTC
            "script".to_string(),
            3,
        ),
        Utxo::new(
            "tx5".to_string(),
            4,
            75_000, // 0.00075 BTC
            "script".to_string(),
            8,
        ),
    ];
    
    let total: u64 = utxos.iter().map(|u| u.amount).sum();
    println!("📊 UTXO 集统计:");
    println!("  总计: {} 个 UTXO", utxos.len());
    println!("  总金额: {} satoshi ({:.8} BTC)", total, total as f64 / 100_000_000.0);
    println!("  金额分布:");
    for (i, utxo) in utxos.iter().enumerate() {
        println!(
            "    #{}: {:>7} sat ({:.8} BTC) - {} 确认",
            i + 1,
            utxo.amount,
            utxo.amount as f64 / 100_000_000.0,
            utxo.confirmations
        );
    }
    println!();
    
    let target_amount = 150_000; // 0.0015 BTC
    let fee_rate = 10; // 10 sat/vbyte
    
    println!("🎯 目标交易:");
    println!("  金额: {} satoshi ({:.8} BTC)", target_amount, target_amount as f64 / 100_000_000.0);
    println!("  费率: {} sat/vbyte", fee_rate);
    println!();
    
    println!("═══════════════════════════════════════════════════════\n");
    
    // 策略 1: 最大优先
    println!("📝 策略 1: 最大优先 (Largest First)");
    println!("───────────────────────────────────────────────────────");
    println!("  适用场景: 快速选择，减少 UTXO 碎片");
    println!();
    
    match UtxoSelector::select(&utxos, target_amount, fee_rate, SelectionStrategy::LargestFirst) {
        Ok((selected, fee)) => {
            print_selection(&selected, fee, target_amount);
        }
        Err(e) => println!("❌ 选择失败: {}", e),
    }
    
    println!();
    
    // 策略 2: 最小优先
    println!("📝 策略 2: 最小优先 (Smallest First)");
    println!("───────────────────────────────────────────────────────");
    println!("  适用场景: 清理小额 UTXO，优化长期费用");
    println!();
    
    match UtxoSelector::select(&utxos, target_amount, fee_rate, SelectionStrategy::SmallestFirst) {
        Ok((selected, fee)) => {
            print_selection(&selected, fee, target_amount);
        }
        Err(e) => println!("❌ 选择失败: {}", e),
    }
    
    println!();
    
    // 策略 3: 最优拟合
    println!("📝 策略 3: 最优拟合 (Best Fit)");
    println!("───────────────────────────────────────────────────────");
    println!("  适用场景: 平衡费用和找零，推荐默认策略");
    println!();
    
    match UtxoSelector::select(&utxos, target_amount, fee_rate, SelectionStrategy::BestFit) {
        Ok((selected, fee)) => {
            print_selection(&selected, fee, target_amount);
        }
        Err(e) => println!("❌ 选择失败: {}", e),
    }
    
    println!();
    
    // 策略 4: 随机选择
    println!("📝 策略 4: 随机选择 (Random)");
    println!("───────────────────────────────────────────────────────");
    println!("  适用场景: 增强隐私性，防止地址关联分析");
    println!();
    
    match UtxoSelector::select(&utxos, target_amount, fee_rate, SelectionStrategy::Random) {
        Ok((selected, fee)) => {
            print_selection(&selected, fee, target_amount);
        }
        Err(e) => println!("❌ 选择失败: {}", e),
    }
    
    println!();
    println!("═══════════════════════════════════════════════════════");
    println!("🎉 UTXO 选择策略演示完成！");
    println!();
    println!("💡 选择建议:");
    println!("  • 日常交易: 使用 BestFit（平衡费用和效率）");
    println!("  • 隐私优先: 使用 Random（防止地址分析）");
    println!("  • 整理钱包: 使用 SmallestFirst（清理碎片）");
    println!("  • 紧急转账: 使用 LargestFirst（快速确认）");
    
    Ok(())
}

#[cfg(feature = "bitcoin")]
fn print_selection(selected: &[Utxo], fee: u64, target: u64) {
    let total: u64 = selected.iter().map(|u| u.amount).sum();
    let change = total.saturating_sub(target + fee);
    
    println!("✅ 选择结果:");
    println!("  选中 UTXO: {} 个", selected.len());
    for (i, utxo) in selected.iter().enumerate() {
        println!(
            "    #{}: {} - {} sat",
            i + 1,
            &utxo.txid[..8],
            utxo.amount
        );
    }
    println!("  总输入: {} sat", total);
    println!("  目标金额: {} sat", target);
    println!("  手续费: {} sat", fee);
    println!("  找零: {} sat", change);
    println!(
        "  效率: {:.2}% (输入利用率)",
        (target as f64 / total as f64) * 100.0
    );
}

#[cfg(not(feature = "bitcoin"))]
fn main() {
    println!("❌ 此示例需要 'bitcoin' feature");
    println!("运行命令: cargo run --example bitcoin_utxo_selection --features bitcoin");
}

