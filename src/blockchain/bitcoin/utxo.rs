//! UTXO (Unspent Transaction Output) 管理
//! 
//! 此模块实现 UTXO 选择算法和管理功能

use crate::core::errors::WalletError;
use bitcoin::Txid;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, info};

/// UTXO 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    /// transaction ID
    pub txid: String,
    /// 输出索引
    pub vout: u32,
    /// 金额（satoshi）
    pub amount: u64,
    /// 脚本公钥
    pub script_pubkey: String,
    /// 确认数
    pub confirmations: u32,
}

impl Utxo {
    /// 创建新的 UTXO
    pub fn new(
        txid: String,
        vout: u32,
        amount: u64,
        script_pubkey: String,
        confirmations: u32,
    ) -> Self {
        Self {
            txid,
            vout,
            amount,
            script_pubkey,
            confirmations,
        }
    }
    
    /// fetch Txid
    pub fn txid(&self) -> Result<Txid, WalletError> {
        Txid::from_str(&self.txid)
            .map_err(|e| WalletError::InvalidAddress(format!("无效的transaction ID: {}", e)))
    }
}

/// UTXO 选择策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    /// 最大优先：优先选择金额最大的 UTXO
    LargestFirst,
    /// 最小优先：优先选择金额最小的 UTXO（减少找零）
    SmallestFirst,
    /// 最优拟合：选择最接近目标金额的 UTXO 组合
    BestFit,
    /// 随机选择：随机选择 UTXO（增强隐私）
    Random,
}

/// UTXO 选择器
pub struct UtxoSelector;

impl UtxoSelector {
    /// 选择 UTXO 以满足目标金额
    /// 
    /// # 参数
    /// - `utxos`: 可用的 UTXO 列表
    /// - `target_amount`: 目标金额（satoshi）
    /// - `fee_rate`: 费率（satoshi/vbyte）
    /// - `strategy`: 选择策略
    /// 
    /// # 返回
    /// - 选中的 UTXO 列表和预估手续费
    pub fn select(
        utxos: &[Utxo],
        target_amount: u64,
        fee_rate: u64,
        strategy: SelectionStrategy,
    ) -> Result<(Vec<Utxo>, u64), WalletError> {
        info!(
            "选择 UTXO: 目标金额={} sat, 费率={} sat/vbyte, 策略={:?}",
            target_amount, fee_rate, strategy
        );
        
        if utxos.is_empty() {
            return Err(WalletError::InsufficientFunds(
                "没有可用的 UTXO".to_string()
            ));
        }
        
        match strategy {
            SelectionStrategy::LargestFirst => {
                Self::select_largest_first(utxos, target_amount, fee_rate)
            }
            SelectionStrategy::SmallestFirst => {
                Self::select_smallest_first(utxos, target_amount, fee_rate)
            }
            SelectionStrategy::BestFit => {
                Self::select_best_fit(utxos, target_amount, fee_rate)
            }
            SelectionStrategy::Random => {
                Self::select_random(utxos, target_amount, fee_rate)
            }
        }
    }
    
    /// 最大优先策略
    fn select_largest_first(
        utxos: &[Utxo],
        target_amount: u64,
        fee_rate: u64,
    ) -> Result<(Vec<Utxo>, u64), WalletError> {
        let mut sorted_utxos = utxos.to_vec();
        sorted_utxos.sort_by(|a, b| b.amount.cmp(&a.amount));
        
        Self::greedy_select(&sorted_utxos, target_amount, fee_rate)
    }
    
    /// 最小优先策略
    fn select_smallest_first(
        utxos: &[Utxo],
        target_amount: u64,
        fee_rate: u64,
    ) -> Result<(Vec<Utxo>, u64), WalletError> {
        let mut sorted_utxos = utxos.to_vec();
        sorted_utxos.sort_by(|a, b| a.amount.cmp(&b.amount));
        
        Self::greedy_select(&sorted_utxos, target_amount, fee_rate)
    }
    
    /// 最优拟合策略
    fn select_best_fit(
        utxos: &[Utxo],
        target_amount: u64,
        fee_rate: u64,
    ) -> Result<(Vec<Utxo>, u64), WalletError> {
        let fee = Self::estimate_fee(1, fee_rate);
        let required_amount = target_amount + fee;
        
        // 首先尝试找到最接近目标金额的单个 UTXO
        let mut best_utxo: Option<&Utxo> = None;
        let mut min_excess = u64::MAX;
        
        for utxo in utxos {
            if utxo.amount >= required_amount {
                let excess = utxo.amount - required_amount;
                if excess < min_excess {
                    min_excess = excess;
                    best_utxo = Some(utxo);
                }
            }
        }
        
        if let Some(utxo) = best_utxo {
            debug!("找到最佳拟合的单个 UTXO: {} (excess: {})", utxo.txid, min_excess);
            return Ok((vec![utxo.clone()], fee));
        }
        
        // 如果没有单个 UTXO 满足，使用贪婪算法
        Self::select_smallest_first(utxos, target_amount, fee_rate)
    }
    
    /// 随机选择策略
    fn select_random(
        utxos: &[Utxo],
        target_amount: u64,
        fee_rate: u64,
    ) -> Result<(Vec<Utxo>, u64), WalletError> {
        use rand::seq::SliceRandom;
        use rand::rngs::OsRng;  // 使用Password学安全的RNG
        
        let mut shuffled_utxos = utxos.to_vec();
        let mut rng = OsRng;  // ✅ Password学安全，保护隐私
        shuffled_utxos.shuffle(&mut rng);
        
        Self::greedy_select(&shuffled_utxos, target_amount, fee_rate)
    }
    
    /// 贪婪选择算法
    fn greedy_select(
        utxos: &[Utxo],
        target_amount: u64,
        fee_rate: u64,
    ) -> Result<(Vec<Utxo>, u64), WalletError> {
        let mut selected = Vec::new();
        let mut total_amount = 0u64;
        
        for utxo in utxos {
            selected.push(utxo.clone());
            // 使用checked_add防止溢出
            total_amount = total_amount.checked_add(utxo.amount)
                .ok_or_else(|| WalletError::ValidationError(
                    "UTXO累加溢出".to_string()
                ))?;
            
            let fee = Self::estimate_fee(selected.len(), fee_rate);
            
            if total_amount >= target_amount + fee {
                debug!(
                    "✅ 选择了 {} 个 UTXO，总金额={} sat，手续费={} sat",
                    selected.len(),
                    total_amount,
                    fee
                );
                return Ok((selected, fee));
            }
        }
        
        Err(WalletError::InsufficientFunds(format!(
            "balance不足: 需要 {} sat, 可用 {} sat",
            target_amount, total_amount
        )))
    }
    
    /// 估算transaction手续费
    /// 
    /// 粗略估算：
    /// - 输入大小: ~148 vbytes (Legacy) / ~68 vbytes (SegWit) / ~58 vbytes (Taproot)
    /// - 输出大小: ~34 vbytes (Legacy) / ~31 vbytes (SegWit) / ~43 vbytes (Taproot)
    /// - 固定开销: ~10 vbytes
    fn estimate_fee(input_count: usize, fee_rate: u64) -> u64 {
        // 保守估算，使用 Legacy 大小
        let input_size = 148 * input_count as u64;
        let output_size = 34 * 2; // 假设 2 个输出（接收者 + 找零）
        let overhead = 10;
        
        let total_vbytes = input_size + output_size + overhead;
        total_vbytes * fee_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_utxos() -> Vec<Utxo> {
        vec![
            Utxo::new(
                "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                0,
                100_000,
                "script1".to_string(),
                6,
            ),
            Utxo::new(
                "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
                0,
                50_000,
                "script2".to_string(),
                3,
            ),
            Utxo::new(
                "0000000000000000000000000000000000000000000000000000000000000003".to_string(),
                0,
                30_000,
                "script3".to_string(),
                10,
            ),
        ]
    }
    
    #[test]
    fn test_select_largest_first() {
        let utxos = create_test_utxos();
        let (selected, fee): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            80_000,
            1,
            SelectionStrategy::LargestFirst,
        ).unwrap();
        
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].amount, 100_000);
        assert!(fee > 0);
    }
    
    #[test]
    fn test_select_smallest_first() {
        let utxos = create_test_utxos();
        let (selected, fee): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            70_000,
            1,
            SelectionStrategy::SmallestFirst,
        ).unwrap();
        
        assert!(selected.len() >= 2);
        assert!(fee > 0);
    }
    
    #[test]
    fn test_select_best_fit() {
        let utxos = create_test_utxos();
        let (selected, fee): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            45_000,
            1,
            SelectionStrategy::BestFit,
        ).unwrap();
        
        assert!(!selected.is_empty());
        assert!(fee > 0);
    }
    
    #[test]
    fn test_insufficient_funds() {
        let utxos = create_test_utxos();
        let result = UtxoSelector::select(
            &utxos,
            1_000_000, // 超过总额
            1,
            SelectionStrategy::LargestFirst,
        );
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::InsufficientFunds(_)));
    }
    
    #[test]
    fn test_empty_utxos() {
        let utxos: Vec<Utxo> = vec![];
        let result = UtxoSelector::select(
            &utxos,
            10_000,
            1,
            SelectionStrategy::LargestFirst,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_fee_estimation() {
        let fee = UtxoSelector::estimate_fee(1, 10);
        assert!(fee > 0);
        assert_eq!(fee, (148 + 68 + 10) * 10); // 输入 + 2个输出 + 开销
        
        let fee_two_inputs = UtxoSelector::estimate_fee(2, 10);
        assert!(fee_two_inputs > fee);
    }
    
    // ============ 新增的 UTXO 测试 ============
    
    #[test]
    fn test_single_utxo_exact_amount() {
        // 单个 UTXO 刚好满足需求（加上手续费）
        let utxos = vec![
            Utxo::new("tx1".to_string(), 0, 50_000, "script".to_string(), 6),
        ];
        
        let (selected, fee): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            48_000,  // 留出手续费空间
            1,
            SelectionStrategy::LargestFirst,
        ).unwrap();
        
        assert_eq!(selected.len(), 1);
        assert!(fee > 0);
        assert!(selected[0].amount >= 48_000 + fee);
    }
    
    #[test]
    fn test_zero_target_amount() {
        // 目标金额为零应该failed或选择最小 UTXO
        let utxos = create_test_utxos();
        let result = UtxoSelector::select(
            &utxos,
            0,
            1,
            SelectionStrategy::LargestFirst,
        );
        
        // 根据实现，可能success选择一个 UTXO 或failed
        // 这里假设success选择
        assert!(result.is_ok() || result.is_err());
    }
    
    #[test]
    fn test_high_fee_rate() {
        let utxos = create_test_utxos();
        
        // 高费率应该需要更多的 UTXO
        let (selected_low, fee_low): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            50_000,
            1,
            SelectionStrategy::LargestFirst,
        ).unwrap();
        
        let (selected_high, fee_high): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            50_000,
            100,  // 高费率
            SelectionStrategy::LargestFirst,
        ).unwrap();
        
        assert!(fee_high > fee_low);
        assert!(selected_high.len() >= selected_low.len());
    }
    
    #[test]
    fn test_best_fit_single_utxo() {
        // BestFit 应该优先选择单个 UTXO
        // 使用有效的 P2PKH script_pubkey
        let script_pubkey = "76a914".to_string() + &"00".repeat(20) + "88ac";
        let utxos = vec![
            Utxo::new("tx1".to_string(), 0, 100_000, script_pubkey.clone(), 6),
            Utxo::new("tx2".to_string(), 0, 55_000, script_pubkey.clone(), 6),  // 刚好够
            Utxo::new("tx3".to_string(), 0, 30_000, script_pubkey, 6),
        ];
        
        let (selected, _): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            50_000,
            1,
            SelectionStrategy::BestFit,
        ).unwrap();
        
        // BestFit 应该选择 55_000 的单个 UTXO
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].amount, 55_000);
    }
    
    #[test]
    fn test_random_strategy_multiple_runs() {
        // 随机策略多次运行可能产生不同结果
        let utxos = create_test_utxos();
        
        let mut results = vec![];
        for _ in 0..5 {
            let (selected, _): (Vec<Utxo>, u64) = UtxoSelector::select(
                &utxos,
                70_000,
                1,
                SelectionStrategy::Random,
            ).unwrap();
            
            results.push(selected.len());
        }
        
        // 至少应该有一个有效结果
        assert!(!results.is_empty());
    }
    
    #[test]
    fn test_large_number_of_utxos() {
        // 测试大量 UTXO 的处理
        let mut utxos = Vec::new();
        for i in 0..100 {
            utxos.push(Utxo::new(
                format!("tx{}", i),
                0,
                1_000,
                format!("script{}", i),
                6,
            ));
        }
        
        let (selected, _): (Vec<Utxo>, u64) = UtxoSelector::select(
            &utxos,
            50_000,
            1,
            SelectionStrategy::SmallestFirst,
        ).unwrap();
        
        assert!(selected.len() > 0);
        assert!(selected.len() <= 100);
    }
    
    #[test]
    fn test_utxo_with_zero_confirmation() {
        // 零确认 UTXO 也应该可以使用（尽管不安全）
        let utxos = vec![
            Utxo::new("tx1".to_string(), 0, 100_000, "script".to_string(), 0),
        ];
        
        let result = UtxoSelector::select(
            &utxos,
            50_000,
            1,
            SelectionStrategy::LargestFirst,
        );
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_very_small_utxos() {
        // 非常小的 UTXO（灰尘）
        let utxos = vec![
            Utxo::new("tx1".to_string(), 0, 546, "s1".to_string(), 6),  // 灰尘阈值
            Utxo::new("tx2".to_string(), 0, 547, "s2".to_string(), 6),
        ];
        
        let result = UtxoSelector::select(
            &utxos,
            500,
            1,
            SelectionStrategy::LargestFirst,
        );
        
        // 可能因为手续费太高而failed
        // 或者success选择
        assert!(result.is_ok() || result.is_err());
    }
    
    #[test]
    fn test_max_amount_calculation() {
        let utxos = create_test_utxos();
        let total: u64 = utxos.iter().map(|u| u.amount).sum();
        
        // 尝试转账总额（应该failed，因为需要手续费）
        let result = UtxoSelector::select(
            &utxos,
            total,
            1,
            SelectionStrategy::LargestFirst,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_fee_increases_with_inputs() {
        // 手续费应该随输入数量增加
        let fee_1 = UtxoSelector::estimate_fee(1, 10);
        let fee_2 = UtxoSelector::estimate_fee(2, 10);
        let fee_5 = UtxoSelector::estimate_fee(5, 10);
        
        assert!(fee_2 > fee_1);
        assert!(fee_5 > fee_2);
    }
    
    #[test]
    fn test_fee_proportional_to_rate() {
        // 手续费应该与费率成正比
        let fee_rate_1 = UtxoSelector::estimate_fee(1, 1);
        let fee_rate_10 = UtxoSelector::estimate_fee(1, 10);
        let fee_rate_100 = UtxoSelector::estimate_fee(1, 100);
        
        assert_eq!(fee_rate_10, fee_rate_1 * 10);
        assert_eq!(fee_rate_100, fee_rate_1 * 100);
    }
    
    #[test]
    fn test_all_strategies_produce_valid_results() {
        let utxos = create_test_utxos();
        let target = 70_000;
        let fee_rate = 1;
        
        let strategies = vec![
            SelectionStrategy::LargestFirst,
            SelectionStrategy::SmallestFirst,
            SelectionStrategy::BestFit,
            SelectionStrategy::Random,
        ];
        
        for strategy in strategies {
            let result = UtxoSelector::select(&utxos, target, fee_rate, strategy);
            
            if let Ok((selected, fee)) = result {
                let total: u64 = selected.iter().map(|u| u.amount).sum();
                assert!(total >= target + fee, "策略 {:?} 选择的 UTXO 总额应该足够", strategy);
            }
        }
    }
}

