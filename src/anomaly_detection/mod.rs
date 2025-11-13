//! 链上异常检测模块（Level 5 - 最优模块化架构）
//! 
//! 使用轻量级 AI 模型实时监控transaction模式，检测可疑活动和防钓鱼攻击。
//! 
//! ## 特性
//! - 🤖 轻量级异常检测（<5MB 内存）
//! - 🎯 实时transaction模式分析
//! - 🛡️ 防钓鱼规则引擎
//! - 🔗 Ethereum + Bitcoin 支持
//! - ⚡ 低延迟推理（<10ms）
//! - 🔌 插件化架构
//! - 📊 事件系统和监控
//! - 💾 灵活的存储后端
//! - ⚙️ 配置驱动和热更新

pub mod detector;
pub mod features;
pub mod rules;
pub mod model;
pub mod config;
pub mod events;
pub mod storage;
pub mod plugins;
pub mod errors;

// ML模块暂未实现，待后续添加
// #[cfg(feature = "ai-anomaly-detection")]
// pub mod ml;

// 核心组件
pub use detector::{AnomalyDetector, DetectionMode};
pub use features::{TransactionFeatures, FeatureExtractor};
pub use rules::{AntiFishingRules, RuleEngine, ThreatLevel};

// Level 5 新增组件
pub use config::AnomalyDetectionConfig;
pub use events::{EventBus, AnomalyEvent, EventSubscriber, LoggingSubscriber, StatisticsSubscriber};
pub use storage::{StorageBackend, DetectionRecord, MemoryStorage, AddressHistory};
pub use plugins::{PluginRegistry, RulePlugin, RuleResult, RecommendedAction, TransactionContext};
pub use errors::{AnomalyDetectionError, Result};

/// 异常检测结果（Level 5 重构版）
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnomalyResult {
    /// 是否检测到异常
    pub is_anomalous: bool,
    /// 异常分数 (0.0-1.0)
    pub score: f64,
    /// 威胁级别
    pub threat_level: ThreatLevel,
    /// 详细原因
    pub reason: String,
    /// 关键特征贡献 (特征名, 贡献度)
    pub key_factors: Vec<(String, f64)>,
}

impl AnomalyResult {
    /// 创建正常结果
    pub fn normal() -> Self {
        Self {
            is_anomalous: false,
            score: 0.0,
            threat_level: ThreatLevel::None,
            reason: "Transaction appears normal".to_string(),
            key_factors: Vec::new(),
        }
    }

    /// 创建异常结果
    pub fn anomalous(score: f64, threat_level: ThreatLevel, reason: String) -> Self {
        Self {
            is_anomalous: true,
            score,
            threat_level,
            reason,
            key_factors: Vec::new(),
        }
    }

    /// 添加关键因素
    pub fn with_factors(mut self, factors: Vec<(String, f64)>) -> Self {
        self.key_factors = factors;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anomaly_result_creation() {
        let normal = AnomalyResult::normal();
        assert!(!normal.is_anomalous);
        assert_eq!(normal.score, 0.0);

        let anomaly = AnomalyResult::anomalous(
            0.95,
            ThreatLevel::High,
            "Suspicious pattern detected".to_string(),
        );
        assert!(anomaly.is_anomalous);
        assert_eq!(anomaly.score, 0.95);
    }
}

