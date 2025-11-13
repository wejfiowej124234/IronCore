//! 异常检测配置模块
//!
//! 提供灵活的配置系统，支持热更新和环境特定配置

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 异常检测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AnomalyDetectionConfig {
    /// 检测模式
    pub mode: DetectionModeConfig,
    
    /// 特征提取配置
    pub feature_extraction: FeatureExtractionConfig,
    
    /// 规则引擎配置
    pub rule_engine: RuleEngineConfig,
    
    /// 模型配置
    pub model: ModelConfig,
    
    /// 存储配置
    pub storage: StorageConfig,
    
    /// 事件配置
    pub events: EventConfig,
    
    /// 性能配置
    pub performance: PerformanceConfig,
}


/// 检测模式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionModeConfig {
    /// 阻止模式 - 阻止所有可疑transaction
    Block,
    /// Warning模式 - 仅记录Warning
    Warn,
    /// 监控模式 - 收集数据但不干预
    Monitor,
}

impl Default for DetectionModeConfig {
    fn default() -> Self {
        Self::Warn
    }
}

/// 特征提取配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureExtractionConfig {
    /// 是否启用特征提取
    pub enabled: bool,
    
    /// 历史数据窗口大小（transaction数量）
    pub history_window_size: usize,
    
    /// 尘埃攻击阈值（单位：lamports 或 wei）
    pub dust_threshold: u64,
    
    /// address年龄窗口（秒）
    pub address_age_window_secs: u64,
    
    /// 自定义特征权重
    pub feature_weights: HashMap<String, f64>,
}

impl Default for FeatureExtractionConfig {
    fn default() -> Self {
        let mut weights = HashMap::new();
        weights.insert("amount".to_string(), 1.0);
        weights.insert("gas_price".to_string(), 1.0);
        weights.insert("contract_interaction".to_string(), 1.5);
        weights.insert("new_address".to_string(), 2.0);
        
        Self {
            enabled: true,
            history_window_size: 1000,
            dust_threshold: 10000, // 0.00001 ETH or minimal amount
            address_age_window_secs: 86400 * 30, // 30 days
            feature_weights: weights,
        }
    }
}

/// 规则引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEngineConfig {
    /// 是否启用规则引擎
    pub enabled: bool,
    
    /// 启用的规则集
    pub enabled_rules: Vec<String>,
    
    /// 黑名单address
    pub blacklist_addresses: Vec<String>,
    
    /// 白名单address
    pub whitelist_addresses: Vec<String>,
    
    /// 高价值转账阈值
    pub high_value_threshold: f64,
    
    /// 异常Gas price偏差比例
    pub abnormal_gas_deviation: f64,
    
    /// 规则权重
    pub rule_weights: HashMap<String, f64>,
    
    /// 自定义规则配置路径
    pub custom_rules_path: Option<PathBuf>,
}

impl Default for RuleEngineConfig {
    fn default() -> Self {
        let mut weights = HashMap::new();
        weights.insert("blacklist".to_string(), 10.0);
        weights.insert("high_value".to_string(), 5.0);
        weights.insert("dust_attack".to_string(), 3.0);
        weights.insert("abnormal_gas".to_string(), 2.0);
        weights.insert("new_address".to_string(), 1.5);
        
        Self {
            enabled: true,
            enabled_rules: vec![
                "blacklist".to_string(),
                "high_value".to_string(),
                "dust_attack".to_string(),
                "abnormal_gas".to_string(),
                "new_address".to_string(),
                "unverified_contract".to_string(),
            ],
            blacklist_addresses: vec![],
            whitelist_addresses: vec![],
            high_value_threshold: 10.0, // 10 ETH
            abnormal_gas_deviation: 2.0, // 2x deviation
            rule_weights: weights,
            custom_rules_path: None,
        }
    }
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 是否启用ML模型
    pub enabled: bool,
    
    /// 模型类型
    pub model_type: ModelType,
    
    /// 模型路径（如果使用预训练模型）
    pub model_path: Option<PathBuf>,
    
    /// 异常检测阈值
    pub anomaly_threshold: f64,
    
    /// 特征缩放配置
    pub feature_scaling: FeatureScalingConfig,
    
    /// 模型超参数
    pub hyperparameters: HashMap<String, f64>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        let mut hyperparameters = HashMap::new();
        hyperparameters.insert("learning_rate".to_string(), 0.01);
        hyperparameters.insert("l2_penalty".to_string(), 0.001);
        
        Self {
            enabled: true,
            model_type: ModelType::Statistical,
            model_path: None,
            anomaly_threshold: 0.7,
            feature_scaling: FeatureScalingConfig::default(),
            hyperparameters,
        }
    }
}

/// 模型类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    /// 统计模型（轻量级）
    Statistical,
    /// 深度学习模型（需要 Candle）
    #[cfg(feature = "ai-anomaly-detection")]
    DeepLearning,
    /// 集成模型
    Ensemble,
}

/// 特征缩放配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureScalingConfig {
    /// 缩放方法
    pub method: ScalingMethod,
    /// 是否自动更新缩放参数
    pub auto_update: bool,
}

impl Default for FeatureScalingConfig {
    fn default() -> Self {
        Self {
            method: ScalingMethod::StandardScaler,
            auto_update: true,
        }
    }
}

/// 缩放方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingMethod {
    /// 标准化 (z-score)
    StandardScaler,
    /// 归一化 (min-max)
    MinMaxScaler,
    /// 不缩放
    None,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// 存储后端类型
    pub backend: StorageBackend,
    
    /// 存储路径
    pub storage_path: PathBuf,
    
    /// 是否启用持久化
    pub enable_persistence: bool,
    
    /// 数据保留期限（天）
    pub retention_days: u64,
    
    /// 缓存大小（条目数）
    pub cache_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Memory,
            storage_path: PathBuf::from("./data/anomaly_detection"),
            enable_persistence: false,
            retention_days: 30,
            cache_size: 10000,
        }
    }
}

/// 存储后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    /// 内存存储（快速，不持久）
    Memory,
    /// SQLite 数据库
    SQLite,
    /// Redis（需要额外配置）
    Redis { url: String },
    /// 自定义后端
    Custom { name: String },
}

/// 事件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// 是否启用事件系统
    pub enabled: bool,
    
    /// 事件缓冲区大小
    pub buffer_size: usize,
    
    /// 是否异步处理事件
    pub async_processing: bool,
    
    /// 事件订阅者
    pub subscribers: Vec<EventSubscriberConfig>,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 1000,
            async_processing: true,
            subscribers: vec![],
        }
    }
}

/// 事件订阅者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscriberConfig {
    /// 订阅者名称
    pub name: String,
    
    /// 订阅的事件类型
    pub event_types: Vec<String>,
    
    /// 是否启用
    pub enabled: bool,
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 最大并发检测数
    pub max_concurrent_detections: usize,
    
    /// 检测超时时间（毫秒）
    pub detection_timeout_ms: u64,
    
    /// 是否启用批处理
    pub enable_batching: bool,
    
    /// 批处理大小
    pub batch_size: usize,
    
    /// 内存限制（MB）
    pub memory_limit_mb: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_detections: 100,
            detection_timeout_ms: 1000,
            enable_batching: false,
            batch_size: 10,
            memory_limit_mb: 5, // 5MB
        }
    }
}

impl AnomalyDetectionConfig {
    /// from文件加载配置
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// 保存配置到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // TODO: Implement loading configuration from environment variables
        Self::default()
    }
    
    /// Validate configuration validity
    pub fn validate(&self) -> Result<(), String> {
        if self.model.anomaly_threshold < 0.0 || self.model.anomaly_threshold > 1.0 {
            return Err("Anomaly threshold must be between 0.0 and 1.0".to_string());
        }
        
        if self.rule_engine.high_value_threshold <= 0.0 {
            return Err("High value threshold must be greater than 0".to_string());
        }
        
        if self.performance.max_concurrent_detections == 0 {
            return Err("Maximum concurrent detections must be greater than 0".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = AnomalyDetectionConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = AnomalyDetectionConfig::default();
        config.model.anomaly_threshold = 1.5;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = AnomalyDetectionConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(!json.is_empty());
        
        let deserialized: AnomalyDetectionConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.validate().is_ok());
    }
}

