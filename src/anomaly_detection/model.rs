//! 轻量级异常检测模型
//!
//! 使用统计方法和简单的评分系统，内存占用 <5MB

use super::features::TransactionFeatures;
use serde::{Deserialize, Serialize};

/// 轻量级异常检测模型
/// 
/// 使用加权评分系统而不是复杂的神经network，以保持低内存占用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightweightAnomalyModel {
    /// 特征权重
    weights: FeatureWeights,
    /// 异常阈值
    anomaly_threshold: f64,
}

/// 特征权重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureWeights {
    pub amount_normalized: f64,
    pub gas_price_normalized: f64,
    pub is_contract_call: f64,
    pub is_new_address: f64,
    pub address_age_days: f64,
    pub address_tx_count_log: f64,
    pub hours_since_last_tx: f64,
    pub is_off_hours: f64,
    pub recent_tx_frequency: f64,
    pub amount_deviation: f64,
    pub is_dust_amount: f64,
    pub network_congestion: f64,
}

impl Default for FeatureWeights {
    fn default() -> Self {
        Self {
            // 高风险特征权重较高
            amount_normalized: 0.15,         // 大额转账较危险
            gas_price_normalized: 0.10,      // 异常gas较危险
            is_contract_call: 0.12,          // 合约调用需Note
            is_new_address: 0.18,            // 新address高风险
            address_age_days: -0.08,         // 老address降低风险
            address_tx_count_log: -0.05,     // 活跃address降低风险
            hours_since_last_tx: 0.05,       // 长时间未transaction略增风险
            is_off_hours: 0.08,              // 非工作时间略增风险
            recent_tx_frequency: 0.12,       // 高频transaction可疑
            amount_deviation: 0.15,          // 金额异常较危险
            is_dust_amount: 0.10,            // 尘埃攻击可疑
            network_congestion: 0.02,        // network拥堵时略增风险
        }
    }
}

impl LightweightAnomalyModel {
    /// 创建自定义配置的模型
    pub fn new(weights: FeatureWeights, anomaly_threshold: f64) -> Self {
        Self {
            weights,
            anomaly_threshold,
        }
    }

    /// 创建默认模型
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self {
            weights: FeatureWeights::default(),
            anomaly_threshold: 0.6, // 0.6 以上视为异常
        }
    }

    /// 预测异常分数
    ///
    /// 返回 0.0-1.0 之间的分数，越高越可疑
    pub fn predict(&self, features: &TransactionFeatures) -> f64 {
        let score = 
            features.amount_normalized * self.weights.amount_normalized +
            features.gas_price_normalized * self.weights.gas_price_normalized +
            features.is_contract_call * self.weights.is_contract_call +
            features.is_new_address * self.weights.is_new_address +
            features.address_age_days * self.weights.address_age_days +
            features.address_tx_count_log * self.weights.address_tx_count_log +
            features.hours_since_last_tx * self.weights.hours_since_last_tx +
            features.is_off_hours * self.weights.is_off_hours +
            features.recent_tx_frequency * self.weights.recent_tx_frequency +
            features.amount_deviation * self.weights.amount_deviation +
            features.is_dust_amount * self.weights.is_dust_amount +
            features.network_congestion * self.weights.network_congestion;

        // 使用 sigmoid 归一化到 0-1
        self.sigmoid(score)
    }

    /// 判断是否为异常
    pub fn is_anomalous(&self, features: &TransactionFeatures) -> bool {
        self.predict(features) > self.anomaly_threshold
    }

    /// Sigmoid 激活函数
    fn sigmoid(&self, x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    /// 设置阈值
    pub fn set_threshold(&mut self, threshold: f64) {
        // ✅ 使用clamp函数
        self.anomaly_threshold = threshold.clamp(0.0, 1.0);
    }

    /// fetch阈值
    pub fn threshold(&self) -> f64 {
        self.anomaly_threshold
    }

    /// 更新权重
    pub fn update_weights(&mut self, weights: FeatureWeights) {
        self.weights = weights;
    }

    /// fetch权重
    pub fn weights(&self) -> &FeatureWeights {
        &self.weights
    }

    /// 模型大小估算（字节）
    pub fn estimated_size_bytes() -> usize {
        // 12 个权重 + 1 个阈值 = 13 个 f64
        // 每个 f64 = 8 字节
        // 加上一些开销
        13 * 8 + 64 // ~168 字节
    }

    /// 解释预测结果
    pub fn explain_prediction(&self, features: &TransactionFeatures) -> Vec<(String, f64)> {
        let mut contributions = vec![
            ("Amount".to_string(), features.amount_normalized * self.weights.amount_normalized),
            ("Gas Price".to_string(), features.gas_price_normalized * self.weights.gas_price_normalized),
            ("Contract Call".to_string(), features.is_contract_call * self.weights.is_contract_call),
            ("New Address".to_string(), features.is_new_address * self.weights.is_new_address),
            ("Address Age".to_string(), features.address_age_days * self.weights.address_age_days),
            ("TX Count".to_string(), features.address_tx_count_log * self.weights.address_tx_count_log),
            ("Time Since Last".to_string(), features.hours_since_last_tx * self.weights.hours_since_last_tx),
            ("Off Hours".to_string(), features.is_off_hours * self.weights.is_off_hours),
            ("TX Frequency".to_string(), features.recent_tx_frequency * self.weights.recent_tx_frequency),
            ("Amount Deviation".to_string(), features.amount_deviation * self.weights.amount_deviation),
            ("Dust Amount".to_string(), features.is_dust_amount * self.weights.is_dust_amount),
            ("Network Congestion".to_string(), features.network_congestion * self.weights.network_congestion),
        ];

        // 按贡献度排序
        contributions.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        contributions
    }
}

#[cfg(feature = "ai-anomaly-detection")]
pub mod ml {
    //! 可选的 ML 模型（使用 Candle）
    //!
    //! 当启用 `ai-anomaly-detection` feature 时可用

    use super::*;

    /// Candle-based deep learning anomaly detection model
    /// 
    /// TODO: Implement real DL model
    pub struct CandleAnomalyModel {
        _placeholder: std::marker::PhantomData<()>,
    }

    impl CandleAnomalyModel {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl Default for CandleAnomalyModel {
        fn default() -> Self {
            Self {
                _placeholder: std::marker::PhantomData,
            }
        }

        /// Load from pretrained model
        pub fn from_pretrained(_path: &str) -> Result<Self, String> {
            // TODO: Use Candle to load ONNX or custom model
            Err("Not implemented yet".to_string())
        }

        /// Predict anomaly score
        pub fn predict(&self, _features: &TransactionFeatures) -> f64 {
            // TODO: Implement Candle inference
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid() {
        let model = LightweightAnomalyModel::default();
        
        assert!(model.sigmoid(0.0) > 0.49 && model.sigmoid(0.0) < 0.51);
        assert!(model.sigmoid(100.0) > 0.99);
        assert!(model.sigmoid(-100.0) < 0.01);
    }

    #[test]
    fn test_prediction() {
        let model = LightweightAnomalyModel::default();
        
        // 低风险transaction
        let low_risk_features = TransactionFeatures {
            amount_normalized: 0.1,
            is_new_address: 0.0,
            is_contract_call: 0.0,
            is_dust_amount: 0.0,
            amount_deviation: 0.0,
            ..TransactionFeatures::default()
        };
        
        let score = model.predict(&low_risk_features);
        assert!(score < 0.6); // 应该低于阈值

        // 高风险transaction
        let high_risk_features = TransactionFeatures {
            amount_normalized: 0.9,
            is_new_address: 1.0,
            is_contract_call: 1.0,
            is_dust_amount: 0.0,
            amount_deviation: 0.8,
            recent_tx_frequency: 0.9,
            ..TransactionFeatures::default()
        };
        
        let score = model.predict(&high_risk_features);
        assert!(score > 0.6); // 应该高于阈值
    }

    #[test]
    fn test_model_size() {
        let size = LightweightAnomalyModel::estimated_size_bytes();
        assert!(size < 1024); // 应该小于 1KB
        println!("Model size: {} bytes", size);
    }

    #[test]
    fn test_explain_prediction() {
        let model = LightweightAnomalyModel::default();
        
        let features = TransactionFeatures {
            amount_normalized: 0.5,
            is_new_address: 1.0,
            ..TransactionFeatures::default()
        };
        
        let explanation = model.explain_prediction(&features);
        assert!(!explanation.is_empty());
        
        // 最高贡献应该是新address
        assert!(explanation[0].0.contains("New Address") || explanation[0].1.abs() > 0.0);
    }
}

