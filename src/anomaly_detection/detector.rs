//! 异常检测器 - 主入口
//!
//! 整合规则引擎、特征提取和ML模型，支持配置、事件和存储

use super::*;
use crate::core::errors::WalletError;
use tracing::{debug, info, warn};
use std::sync::Arc;
use std::time::Instant;
use chrono::Utc;

use crate::anomaly_detection::{
    config::AnomalyDetectionConfig,
    events::{EventBus, AnomalyEvent},
    storage::{StorageBackend, DetectionRecord, MemoryStorage},
    plugins::PluginRegistry,
};

/// 异常检测器（重构版 - Level 5）
pub struct AnomalyDetector {
    /// 配置
    config: AnomalyDetectionConfig,
    /// 特征提取器
    feature_extractor: features::FeatureExtractor,
    /// 规则引擎
    rule_engine: rules::RuleEngine,
    /// ML 模型
    model: model::LightweightAnomalyModel,
    /// 插件注册中心
    plugin_registry: Arc<PluginRegistry>,
    /// 事件总线
    event_bus: Arc<EventBus>,
    /// 存储后端
    storage: Arc<dyn StorageBackend>,
    /// 是否启用
    enabled: bool,
    /// 检测模式
    mode: DetectionMode,
}

/// 检测模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMode {
    /// 仅Warning（记录日志但不阻止）
    WarnOnly,
    /// 阻止高风险transaction
    BlockHighRisk,
    /// 阻止所有可疑transaction
    BlockAll,
}

impl AnomalyDetector {
    /// 创建新的检测器（带配置）
    pub fn with_config(config: AnomalyDetectionConfig) -> Self {
        info!("🤖 初始化 AI 异常检测系统（Level 5 架构）");
        
        // 初始化存储后端
        let storage: Arc<dyn StorageBackend> = Arc::new(MemoryStorage::new(
            config.storage.cache_size
        ));
        
        // 初始化事件总线
        let event_bus = Arc::new(EventBus::new(config.events.buffer_size));
        
        // 初始化插件注册中心
        let plugin_registry = Arc::new(PluginRegistry::new());
        
        // 注册内置插件
        Self::register_builtin_plugins(&plugin_registry, &config);
        
        Self {
            config: config.clone(),
            feature_extractor: features::FeatureExtractor::new(),
            rule_engine: rules::RuleEngine::default(),
            model: model::LightweightAnomalyModel::default(),
            plugin_registry,
            event_bus,
            storage,
            enabled: true,
            mode: DetectionMode::BlockHighRisk,
        }
    }
    
    /// 创建新的检测器（使用默认配置）
    pub fn new() -> Self {
        Self::with_config(AnomalyDetectionConfig::default())
    }

    /// 创建默认检测器
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new()
    }
    
    /// 注册内置插件
    fn register_builtin_plugins(registry: &PluginRegistry, config: &AnomalyDetectionConfig) {
        use crate::anomaly_detection::plugins::*;
        
        if config.rule_engine.enabled_rules.contains(&"blacklist".to_string()) {
            let plugin = Arc::new(BlacklistPlugin::new(
                config.rule_engine.blacklist_addresses.clone()
            ));
            let _ = registry.register(plugin);
        }
        
        if config.rule_engine.enabled_rules.contains(&"high_value".to_string()) {
            let plugin = Arc::new(HighValuePlugin::new(
                config.rule_engine.high_value_threshold
            ));
            let _ = registry.register(plugin);
        }
        
        if config.rule_engine.enabled_rules.contains(&"dust_attack".to_string()) {
            let _ = registry.register(Arc::new(DustAttackPlugin::new()));
        }
        
        if config.rule_engine.enabled_rules.contains(&"new_address".to_string()) {
            let _ = registry.register(Arc::new(NewAddressPlugin::new()));
        }
        
        info!("✅ 已注册 {} 个内置插件", registry.count());
    }

    /// 检测transaction异常（重构版 - 集成插件和事件）
    ///
    /// # 参数
    /// - `to_address`: Recipient address
    /// - `amount`: 转账金额（原生代币单位）
    /// - `gas_price`: Gas 价格（可选）
    /// - `is_contract`: 是否为合约调用
    ///
    /// # 返回
    /// - `AnomalyResult`: 检测结果
    pub fn detect_transaction(
        &mut self,
        to_address: &str,
        amount: f64,
        gas_price: Option<u64>,
        is_contract: bool,
    ) -> AnomalyResult {
        let start_time = Instant::now();
        // 使用SHA-256生成Transaction hash（安全哈希算法）
        use sha2::{Sha256, Digest};
        let tx_hash = format!("{:x}", Sha256::digest(format!("{}{}{:?}", to_address, amount, gas_price).as_bytes()));
        
        // 发布检测start事件
        if self.config.events.enabled {
            self.event_bus.publish(AnomalyEvent::DetectionStarted {
                transaction_hash: tx_hash.clone(),
                timestamp: Utc::now(),
            });
        }
        
        if !self.enabled {
            debug!("异常检测已禁用");
            return AnomalyResult::normal();
        }

        info!("🔍 检测transaction: to={}, amount={}, gas={:?}", 
            to_address, amount, gas_price);

        // 1. 提取特征
        let features = self.feature_extractor.extract(
            to_address,
            amount,
            gas_price,
            is_contract,
        );

        // 2. 规则引擎评估
        let (rule_threat_level, triggered_rules) = self.rule_engine.evaluate_transaction(
            to_address,
            amount,
            gas_price,
            is_contract,
        );
        
        // 2.5 插件评估
        let context = crate::anomaly_detection::TransactionContext::new(features.clone(), amount)
            .with_addresses(Some(to_address.to_string()), None)
            .with_gas_price(gas_price);
        let plugin_results = self.plugin_registry.evaluate_all(&context);
        let mut plugin_threat_level = ThreatLevel::None;
        let mut plugin_reasons = Vec::new();
        
        for (name, result) in &plugin_results {
            if result.triggered {
                plugin_threat_level = Self::max_threat_level(plugin_threat_level, result.threat_level);
                plugin_reasons.push(format!("{}({})", name, result.reason));
                
                // 发布规则触发事件
                if self.config.events.enabled {
                    self.event_bus.publish(AnomalyEvent::RuleTriggered {
                        transaction_hash: tx_hash.clone(),
                        rule_name: name.clone(),
                        threat_level: result.threat_level,
                        details: result.reason.clone(),
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        // 3. ML 模型预测
        let ml_score = self.model.predict(&features);
        let ml_threat_level = ThreatLevel::from_score(ml_score);
        
        // 发布模型预测事件
        if self.config.events.enabled {
            self.event_bus.publish(AnomalyEvent::ModelPrediction {
                transaction_hash: tx_hash.clone(),
                score: ml_score,
                features: vec![], // TODO: Extract feature vectors
                timestamp: Utc::now(),
            });
        }

        // 4. 综合判断（取所有来源的最高威胁级别）
        let final_threat_level = Self::max_threat_level(
            Self::max_threat_level(rule_threat_level, ml_threat_level),
            plugin_threat_level
        );
        let final_score = ml_score
            .max(Self::threat_level_to_score(rule_threat_level))
            .max(Self::threat_level_to_score(plugin_threat_level));

        // 5. 生成结果
        let is_anomalous = match self.mode {
            DetectionMode::WarnOnly => false, // 仅Warning模式不阻止
            DetectionMode::BlockHighRisk => {
                matches!(final_threat_level, ThreatLevel::High | ThreatLevel::Critical)
            }
            DetectionMode::BlockAll => {
                final_threat_level != ThreatLevel::None
            }
        };

        let reason = self.generate_reason(&triggered_rules, &plugin_reasons, ml_score, &features);
        
        let key_factors = self.model.explain_prediction(&features);

        let result = AnomalyResult {
            is_anomalous,
            score: final_score,
            threat_level: final_threat_level,
            reason: reason.clone(),
            key_factors,
        };

        // 记录结果到存储（内存存储始终可用，enable_persistence控制是否持久化到磁盘）
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let record = DetectionRecord {
            transaction_hash: tx_hash.clone(),
            result: result.clone(),
            timestamp: Utc::now(),
            duration_ms,
            blockchain: "unknown".to_string(),
            metadata: std::collections::HashMap::new(),
        };
        let _ = self.storage.save_record(record);
        
        // 发布检测completed事件
        if self.config.events.enabled {
            self.event_bus.publish(AnomalyEvent::DetectionCompleted {
                transaction_hash: tx_hash.clone(),
                result: result.clone(),
                duration_ms,
                timestamp: Utc::now(),
            });
        }
        
        // 发布阻止/Warning事件
        if is_anomalous && self.config.events.enabled {
            if self.should_block(&final_threat_level) {
                self.event_bus.publish(AnomalyEvent::TransactionBlocked {
                    transaction_hash: tx_hash.clone(),
                    reason: reason.clone(),
                    threat_level: final_threat_level,
                    timestamp: Utc::now(),
                });
            } else {
                self.event_bus.publish(AnomalyEvent::WarningIssued {
                    transaction_hash: tx_hash,
                    message: reason.clone(),
                    threat_level: final_threat_level,
                    timestamp: Utc::now(),
                });
            }
        }

        // 记录结果
        if is_anomalous {
            warn!("⚠️ 检测到异常transaction: threat_level={:?}, score={:.2}, plugins={}, rules={:?}",
                final_threat_level, final_score, plugin_reasons.len(), triggered_rules);
        } else {
            debug!("✅ transaction正常: score={:.2}, 耗时={}ms", final_score, duration_ms);
        }

        result
    }

    /// validatetransaction（返回 Result）
    pub fn validate_transaction(
        &mut self,
        to_address: &str,
        amount: f64,
        gas_price: Option<u64>,
        is_contract: bool,
    ) -> std::result::Result<(), WalletError> {
        let result = self.detect_transaction(to_address, amount, gas_price, is_contract);

        if result.is_anomalous && self.should_block(&result.threat_level) {
            Err(WalletError::ValidationError(format!(
                "Transaction blocked by anomaly detection: {} (threat_level={:?}, score={:.2})",
                result.reason, result.threat_level, result.score
            )))
        } else if result.threat_level != ThreatLevel::None {
            // 即使不阻止，也记录Warning
            warn!("⚠️ 可疑transaction被允许: {}", result.reason);
            Ok(())
        } else {
            Ok(())
        }
    }

    /// 判断是否应该阻止transaction
    fn should_block(&self, threat_level: &ThreatLevel) -> bool {
        match self.mode {
            DetectionMode::WarnOnly => false,
            DetectionMode::BlockHighRisk => {
                matches!(threat_level, ThreatLevel::High | ThreatLevel::Critical)
            }
            DetectionMode::BlockAll => *threat_level != ThreatLevel::None,
        }
    }

    /// 生成原因描述（重构版 - 包含插件）
    fn generate_reason(
        &self,
        triggered_rules: &[String],
        plugin_reasons: &[String],
        ml_score: f64,
        features: &TransactionFeatures,
    ) -> String {
        let mut parts = Vec::new();

        if !triggered_rules.is_empty() {
            parts.push(format!("Triggered {} rule(s): {}", 
                triggered_rules.len(), 
                triggered_rules.join(", ")));
        }
        
        if !plugin_reasons.is_empty() {
            parts.push(format!("Plugin alerts: {}", plugin_reasons.join(", ")));
        }

        if ml_score > 0.6 {
            parts.push(format!("ML anomaly score: {:.2}", ml_score));
            
            // 添加主要风险因素
            let top_factors = self.model.explain_prediction(features)
                .into_iter()
                .take(3)
                .filter(|(_, score)| score.abs() > 0.05)
                .map(|(name, score)| format!("{}({:.2})", name, score))
                .collect::<Vec<_>>();
            
            if !top_factors.is_empty() {
                parts.push(format!("Key factors: {}", top_factors.join(", ")));
            }
        }

        if parts.is_empty() {
            "No anomalies detected".to_string()
        } else {
            parts.join("; ")
        }
    }

    /// fetch更高的威胁级别
    fn max_threat_level(a: ThreatLevel, b: ThreatLevel) -> ThreatLevel {
        use ThreatLevel::*;
        match (a, b) {
            (Critical, _) | (_, Critical) => Critical,
            (High, _) | (_, High) => High,
            (Medium, _) | (_, Medium) => Medium,
            (Low, _) | (_, Low) => Low,
            _ => None,
        }
    }

    /// 威胁级别转分数
    fn threat_level_to_score(level: ThreatLevel) -> f64 {
        match level {
            ThreatLevel::None => 0.0,
            ThreatLevel::Low => 0.3,
            ThreatLevel::Medium => 0.5,
            ThreatLevel::High => 0.75,
            ThreatLevel::Critical => 0.95,
        }
    }

    // === 配置方法 ===

    /// 启用检测器
    pub fn enable(&mut self) {
        self.enabled = true;
        info!("🟢 异常检测已启用");
    }

    /// 禁用检测器
    pub fn disable(&mut self) {
        self.enabled = false;
        warn!("🔴 异常检测已禁用");
    }

    /// 设置检测模式
    pub fn set_mode(&mut self, mode: DetectionMode) {
        self.mode = mode;
        info!("🔧 检测模式设置为: {:?}", mode);
    }

    /// fetch检测模式
    pub fn mode(&self) -> DetectionMode {
        self.mode
    }

    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 更新模型阈值
    pub fn set_model_threshold(&mut self, threshold: f64) {
        self.model.set_threshold(threshold);
        info!("🔧 ML 模型阈值设置为: {:.2}", threshold);
    }

    /// 添加address到黑名单
    pub fn add_to_blacklist(&mut self, address: String, reason: String) {
        self.rule_engine.rules_mut().add_to_blacklist(address.clone(), reason.clone());
        info!("🚫 address已加入黑名单: {} ({})", address, reason);
    }

    /// fetch统计信息
    pub fn get_stats(&self) -> DetectorStats {
        DetectorStats {
            history_size: self.feature_extractor.history_size(),
            model_threshold: self.model.threshold(),
            is_enabled: self.enabled,
            mode: self.mode,
        }
    }

    /// 清空历史
    pub fn clear_history(&mut self) {
        self.feature_extractor.clear_history();
        info!("🧹 历史数据已清空");
    }
    
    // === 新增：Level 5 架构接口 ===
    
    /// fetch配置（只读）
    pub fn config(&self) -> &AnomalyDetectionConfig {
        &self.config
    }
    
    /// fetch事件总线
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }
    
    /// fetch插件注册中心
    pub fn plugin_registry(&self) -> Arc<PluginRegistry> {
        self.plugin_registry.clone()
    }
    
    /// fetch存储后端
    pub fn storage(&self) -> Arc<dyn StorageBackend> {
        self.storage.clone()
    }
    
    /// 更新配置（热更新）
    pub fn update_config(&mut self, config: AnomalyDetectionConfig) -> std::result::Result<(), String> {
        config.validate()?;
        
        // 重新注册插件
        let new_registry = PluginRegistry::new();
        Self::register_builtin_plugins(&new_registry, &config);
        self.plugin_registry = Arc::new(new_registry);
        
        self.config = config;
        
        // 发布配置更新事件
        if self.config.events.enabled {
            self.event_bus.publish(AnomalyEvent::ConfigurationUpdated {
                changes: std::collections::HashMap::new(),
                timestamp: Utc::now(),
            });
        }
        
        info!("✅ 配置已更新并validate");
        Ok(())
    }
    
    /// 导出配置到文件
    pub fn export_config(&self, path: &std::path::PathBuf) -> std::result::Result<(), String> {
        self.config.save_to_file(path)
            .map_err(|e| format!("导出配置failed: {}", e))
    }
    
    /// from文件导入配置
    pub fn import_config(&mut self, path: &std::path::PathBuf) -> std::result::Result<(), String> {
        let config = AnomalyDetectionConfig::from_file(path)
            .map_err(|e| format!("导入配置failed: {}", e))?;
        self.update_config(config)
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// 检测器统计信息
#[derive(Debug, Clone)]
pub struct DetectorStats {
    pub history_size: usize,
    pub model_threshold: f64,
    pub is_enabled: bool,
    pub mode: DetectionMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = AnomalyDetector::new();
        assert!(detector.is_enabled());
        assert_eq!(detector.mode(), DetectionMode::BlockHighRisk);
    }

    #[test]
    fn test_normal_transaction() {
        let mut detector = AnomalyDetector::new();
        
        let result = detector.detect_transaction(
            "0x1234567890123456789012345678901234567890",
            1.0,
            Some(100_000_000_000),
            false,
        );
        
        // 第一次transaction可能有Warning，但不应该阻止
        assert!(result.threat_level != ThreatLevel::Critical);
    }

    #[test]
    fn test_blacklisted_address() {
        let mut detector = AnomalyDetector::new();
        
        let result = detector.detect_transaction(
            "0x0000000000000000000000000000000000000000",
            1.0,
            None,
            false,
        );
        
        assert!(result.is_anomalous);
        assert_eq!(result.threat_level, ThreatLevel::Critical);
    }

    #[test]
    fn test_warn_only_mode() {
        let mut detector = AnomalyDetector::new();
        detector.set_mode(DetectionMode::WarnOnly);
        
        // 即使是黑名单address，也不应该阻止
        let result = detector.detect_transaction(
            "0x0000000000000000000000000000000000000000",
            1.0,
            None,
            false,
        );
        
        assert!(!result.is_anomalous); // WarnOnly 模式不阻止
        assert!(result.threat_level != ThreatLevel::None); // 但会标记威胁
    }

    #[test]
    fn test_validate_transaction() {
        let mut detector = AnomalyDetector::new();
        
        // 正常transaction应该通过
        let result = detector.validate_transaction(
            "0x1234567890123456789012345678901234567890",
            1.0,
            None,
            false,
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_blacklist() {
        let mut detector = AnomalyDetector::new();
        
        detector.add_to_blacklist(
            "0xBADBADBADBADBADBADBADBADBADBADBADBADBAD".to_string(),
            "Known phishing address".to_string(),
        );
        
        let result = detector.detect_transaction(
            "0xBADBADBADBADBADBADBADBADBADBADBADBADBAD",
            1.0,
            None,
            false,
        );
        
        assert!(result.is_anomalous);
        assert_eq!(result.threat_level, ThreatLevel::Critical);
    }
}

