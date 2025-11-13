//! transaction特征提取器
//!
//! from区块链transaction中提取用于异常检测的特征

use serde::{Deserialize, Serialize};

/// transaction特征向量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFeatures {
    // === 基础特征 ===
    /// 转账金额（归一化到 0-1）
    pub amount_normalized: f64,
    /// Gas 价格（归一化）
    pub gas_price_normalized: f64,
    /// 是否为合约调用
    pub is_contract_call: f64, // 0.0 or 1.0
    
    // === address特征 ===
    /// Recipient address是否为新address
    pub is_new_address: f64,
    /// address年龄（天数，归一化）
    pub address_age_days: f64,
    /// address历史transaction数量（对数归一化）
    pub address_tx_count_log: f64,
    
    // === 时间特征 ===
    /// 距离上次transaction的时间（小时，归一化）
    pub hours_since_last_tx: f64,
    /// 是否在非工作时间（0.0-1.0）
    pub is_off_hours: f64,
    
    // === 模式特征 ===
    /// 近期transaction频率（每小时transaction数）
    pub recent_tx_frequency: f64,
    /// 金额变化幅度（相对于历史平均）
    pub amount_deviation: f64,
    /// 是否为小额转账（尘埃）
    pub is_dust_amount: f64,
    
    // === network特征 ===
    /// network拥堵程度（0-1）
    pub network_congestion: f64,
}

impl TransactionFeatures {
    /// 创建默认特征（全零）
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self {
            amount_normalized: 0.0,
            gas_price_normalized: 0.0,
            is_contract_call: 0.0,
            is_new_address: 0.0,
            address_age_days: 0.0,
            address_tx_count_log: 0.0,
            hours_since_last_tx: 0.0,
            is_off_hours: 0.0,
            recent_tx_frequency: 0.0,
            amount_deviation: 0.0,
            is_dust_amount: 0.0,
            network_congestion: 0.0,
        }
    }

    /// 转换为向量
    pub fn to_vector(&self) -> Vec<f64> {
        vec![
            self.amount_normalized,
            self.gas_price_normalized,
            self.is_contract_call,
            self.is_new_address,
            self.address_age_days,
            self.address_tx_count_log,
            self.hours_since_last_tx,
            self.is_off_hours,
            self.recent_tx_frequency,
            self.amount_deviation,
            self.is_dust_amount,
            self.network_congestion,
        ]
    }

    /// from向量创建
    pub fn from_vector(vec: &[f64]) -> Option<Self> {
        if vec.len() < 12 {
            return None;
        }

        Some(Self {
            amount_normalized: vec[0],
            gas_price_normalized: vec[1],
            is_contract_call: vec[2],
            is_new_address: vec[3],
            address_age_days: vec[4],
            address_tx_count_log: vec[5],
            hours_since_last_tx: vec[6],
            is_off_hours: vec[7],
            recent_tx_frequency: vec[8],
            amount_deviation: vec[9],
            is_dust_amount: vec[10],
            network_congestion: vec[11],
        })
    }

    /// 特征维度
    pub fn dimension() -> usize {
        12
    }
}

/// 特征提取器
pub struct FeatureExtractor {
    /// 历史transaction数据（用于计算统计特征）
    history: Vec<HistoricalTransaction>,
    /// 最大金额（用于归一化）
    max_amount: f64,
    /// 最大 gas 价格（用于归一化）
    max_gas_price: u64,
}

#[derive(Debug, Clone)]
pub struct HistoricalTransaction {
    pub to_address: String,
    pub amount: f64,
    pub gas_price: u64,
    pub timestamp: u64,
    pub is_contract: bool,
}

impl FeatureExtractor {
    /// 创建新的特征提取器
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_amount: 1000.0,      // 默认最大 1000 ETH
            max_gas_price: 1_000_000_000_000, // 1000 Gwei
        }
    }

    /// 提取transaction特征
    pub fn extract(
        &mut self,
        to_address: &str,
        amount: f64,
        gas_price: Option<u64>,
        is_contract: bool,
    ) -> TransactionFeatures {
        let now = Self::current_timestamp();
        
        // 基础特征
        let amount_normalized = (amount / self.max_amount).min(1.0);
        let gas_price_normalized = if let Some(gp) = gas_price {
            (gp as f64 / self.max_gas_price as f64).min(1.0)
        } else {
            0.0
        };
        let is_contract_call = if is_contract { 1.0 } else { 0.0 };

        // address特征
        let is_new_address = if self.has_seen_address(to_address) { 0.0 } else { 1.0 };
        let address_age_days = self.get_address_age_days(to_address);
        let address_tx_count = self.get_address_tx_count(to_address);
        let address_tx_count_log = if address_tx_count > 0 {
            (address_tx_count as f64).ln() / 10.0 // 归一化到 0-1 范围
        } else {
            0.0
        };

        // 时间特征
        let last_tx_time = self.get_last_tx_timestamp();
        let hours_since_last_tx = if let Some(last) = last_tx_time {
            ((now - last) as f64 / 3600.0).min(24.0) / 24.0 // 最多 24 小时
        } else {
            1.0 // 如果没有历史，设为最大值
        };
        let is_off_hours = Self::is_off_hours_time(now);

        // 模式特征
        let recent_tx_frequency = self.calculate_recent_frequency(3600); // 1 小时内
        let amount_deviation = self.calculate_amount_deviation(amount);
        let is_dust_amount = if amount < 0.001 { 1.0 } else { 0.0 };

        // Network features (simplified version, should fetch from blockchain in production)
        let network_congestion = 0.5; // TODO: Fetch actual congestion data from blockchain

        // Record transaction
        self.record_transaction(HistoricalTransaction {
            to_address: to_address.to_string(),
            amount,
            gas_price: gas_price.unwrap_or(0),
            timestamp: now,
            is_contract,
        });

        TransactionFeatures {
            amount_normalized,
            gas_price_normalized,
            is_contract_call,
            is_new_address,
            address_age_days,
            address_tx_count_log,
            hours_since_last_tx,
            is_off_hours,
            recent_tx_frequency,
            amount_deviation,
            is_dust_amount,
            network_congestion,
        }
    }

    /// fetch当前时间戳
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs()
    }

    /// check是否见过该address
    fn has_seen_address(&self, address: &str) -> bool {
        self.history.iter().any(|tx| tx.to_address == address)
    }

    /// fetchaddress年龄（天数）
    fn get_address_age_days(&self, address: &str) -> f64 {
        let now = Self::current_timestamp();
        
        if let Some(first_tx) = self.history.iter().find(|tx| tx.to_address == address) {
            let age_seconds = now - first_tx.timestamp;
            let age_days = age_seconds as f64 / 86400.0;
            (age_days / 365.0).min(1.0) // 归一化到 1 年
        } else {
            0.0
        }
    }

    /// fetchaddresstransaction数量
    fn get_address_tx_count(&self, address: &str) -> usize {
        self.history.iter().filter(|tx| tx.to_address == address).count()
    }

    /// fetch最后一次transaction时间
    fn get_last_tx_timestamp(&self) -> Option<u64> {
        self.history.last().map(|tx| tx.timestamp)
    }

    /// 判断是否为非工作时间
    fn is_off_hours_time(timestamp: u64) -> f64 {
        use chrono::{DateTime, Datelike, Timelike, Utc};
        
        let dt: DateTime<Utc> = DateTime::from_timestamp(timestamp as i64, 0)
            .unwrap_or_else(|| DateTime::UNIX_EPOCH);
        let hour = dt.hour();
        let weekday = dt.weekday();
        
        // 非工作时间: 22:00 - 06:00 或周末
        if !(6..22).contains(&hour) || weekday.num_days_from_monday() >= 5 {
            1.0
        } else {
            0.0
        }
    }

    /// 计算近期transaction频率
    fn calculate_recent_frequency(&self, time_window: u64) -> f64 {
        let now = Self::current_timestamp();
        let count = self
            .history
            .iter()
            .filter(|tx| now - tx.timestamp < time_window)
            .count();
        
        // 归一化：每小时 10 笔以上视为高频
        (count as f64 / 10.0).min(1.0)
    }

    /// 计算金额偏差
    fn calculate_amount_deviation(&self, amount: f64) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        let avg_amount: f64 = self.history.iter().map(|tx| tx.amount).sum::<f64>() 
            / self.history.len() as f64;
        
        if avg_amount < 0.001 {
            return 0.0;
        }

        // 相对偏差
        let deviation = (amount - avg_amount).abs() / avg_amount;
        (deviation / 10.0).min(1.0) // 10倍偏差为最大
    }

    /// 记录transaction
    fn record_transaction(&mut self, tx: HistoricalTransaction) {
        const MAX_HISTORY: usize = 1000;
        
        self.history.push(tx);
        
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
    }

    /// 清空历史
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// fetch历史transaction数量
    pub fn history_size(&self) -> usize {
        self.history.len()
    }
}

impl Default for FeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_vector_conversion() {
        let features = TransactionFeatures::default();
        let vec = features.to_vector();
        
        assert_eq!(vec.len(), TransactionFeatures::dimension());
        
        let reconstructed = TransactionFeatures::from_vector(&vec).unwrap();
        assert_eq!(reconstructed.amount_normalized, 0.0);
    }

    #[test]
    fn test_feature_extraction() {
        let mut extractor = FeatureExtractor::new();
        
        let features = extractor.extract(
            "0x1234567890123456789012345678901234567890",
            5.0,
            Some(100_000_000_000),
            false,
        );
        
        assert!(features.amount_normalized > 0.0);
        assert_eq!(features.is_new_address, 1.0); // 第一次见到
        assert_eq!(features.is_contract_call, 0.0);
    }

    #[test]
    fn test_address_tracking() {
        let mut extractor = FeatureExtractor::new();
        
        // 第一次transaction
        let features1 = extractor.extract("0xABCD", 1.0, None, false);
        assert_eq!(features1.is_new_address, 1.0);
        
        // 第二次transaction（同一address）
        let features2 = extractor.extract("0xABCD", 2.0, None, false);
        assert_eq!(features2.is_new_address, 0.0);
    }

    #[test]
    fn test_dust_detection() {
        let mut extractor = FeatureExtractor::new();
        
        // 尘埃transaction
        let features = extractor.extract("0xDUST", 0.0001, None, false);
        assert_eq!(features.is_dust_amount, 1.0);
        
        // 正常transaction
        let features = extractor.extract("0xNORMAL", 1.0, None, false);
        assert_eq!(features.is_dust_amount, 0.0);
    }
}

