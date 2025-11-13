//! 规则插件系统
//!
//! 提供可扩展的规则插件架构，允许动态加载和管理检测规则

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::anomaly_detection::{TransactionFeatures, ThreatLevel};

/// transaction上下文（包含原始数据和特征）
#[derive(Debug, Clone)]
pub struct TransactionContext {
    /// 归一化特征
    pub features: TransactionFeatures,
    /// 原始转账金额
    pub amount: f64,
    /// 目标address（可选）
    pub to_address: Option<String>,
    /// 来源address（可选）
    pub from_address: Option<String>,
    /// Gas 价格（可选）
    pub gas_price: Option<u64>,
}

impl TransactionContext {
    pub fn new(features: TransactionFeatures, amount: f64) -> Self {
        Self {
            features,
            amount,
            to_address: None,
            from_address: None,
            gas_price: None,
        }
    }
    
    pub fn with_addresses(mut self, to: Option<String>, from: Option<String>) -> Self {
        self.to_address = to;
        self.from_address = from;
        self
    }
    
    pub fn with_gas_price(mut self, gas_price: Option<u64>) -> Self {
        self.gas_price = gas_price;
        self
    }
}

/// 规则插件 trait
pub trait RulePlugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;
    
    /// 插件版本
    fn version(&self) -> &str;
    
    /// 插件描述
    fn description(&self) -> &str;
    
    /// 评估transaction
    fn evaluate(&self, context: &TransactionContext) -> RuleResult;
    
    /// 插件权重（用于综合评分）
    fn weight(&self) -> f64 {
        1.0
    }
    
    /// 是否启用
    fn is_enabled(&self) -> bool {
        true
    }
}

/// 规则评估结果
#[derive(Debug, Clone)]
pub struct RuleResult {
    /// 是否触发规则
    pub triggered: bool,
    
    /// 威胁级别
    pub threat_level: ThreatLevel,
    
    /// 原因说明
    pub reason: String,
    
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
    
    /// 建议的行动
    pub recommended_action: RecommendedAction,
}

impl RuleResult {
    pub fn no_threat() -> Self {
        Self {
            triggered: false,
            threat_level: ThreatLevel::None,
            reason: "未检测到威胁".to_string(),
            confidence: 1.0,
            recommended_action: RecommendedAction::Allow,
        }
    }
    
    pub fn threat(
        threat_level: ThreatLevel,
        reason: impl Into<String>,
        confidence: f64,
        action: RecommendedAction,
    ) -> Self {
        Self {
            triggered: true,
            threat_level,
            reason: reason.into(),
            confidence,
            recommended_action: action,
        }
    }
}

/// 建议的行动
#[derive(Debug, Clone, PartialEq)]
pub enum RecommendedAction {
    /// 允许transaction
    Allow,
    /// 发出Warning
    Warn,
    /// 阻止transaction
    Block,
    /// 需要人工审核
    RequireApproval,
}

/// 插件注册中心
pub struct PluginRegistry {
    plugins: Arc<Mutex<HashMap<String, Arc<dyn RulePlugin>>>>,
}

impl PluginRegistry {
    /// 创建新的注册中心
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// 注册插件
    pub fn register(&self, plugin: Arc<dyn RulePlugin>) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        
        let name = plugin.name().to_string();
        if plugins.contains_key(&name) {
            return Err(format!("Plugin '{}' is already registered", name));
        }
        
        plugins.insert(name.clone(), plugin);
        println!("[Plugin System] Registered plugin: {}", name);
        Ok(())
    }
    
    /// Cancel注册插件
    pub fn unregister(&self, plugin_name: &str) -> Result<(), String> {
        let mut plugins = self.plugins.lock().unwrap();
        
        if plugins.remove(plugin_name).is_some() {
            println!("[插件系统] 已卸载插件: {}", plugin_name);
            Ok(())
        } else {
            Err(format!("插件 '{}' 未找到", plugin_name))
        }
    }
    
    /// fetch插件
    pub fn get_plugin(&self, plugin_name: &str) -> Option<Arc<dyn RulePlugin>> {
        let plugins = self.plugins.lock().unwrap();
        plugins.get(plugin_name).cloned()
    }
    
    /// fetch所有插件
    pub fn get_all_plugins(&self) -> Vec<Arc<dyn RulePlugin>> {
        let plugins = self.plugins.lock().unwrap();
        plugins.values().cloned().collect()
    }
    
    /// 评估所有插件
    pub fn evaluate_all(&self, context: &TransactionContext) -> Vec<(String, RuleResult)> {
        let plugins = self.plugins.lock().unwrap();
        
        plugins
            .iter()
            .filter(|(_, plugin)| plugin.is_enabled())
            .map(|(name, plugin)| {
                let result = plugin.evaluate(context);
                (name.clone(), result)
            })
            .collect()
    }
    
    /// fetch插件数量
    pub fn count(&self) -> usize {
        let plugins = self.plugins.lock().unwrap();
        plugins.len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 内置插件 ====================

/// 黑名单插件
pub struct BlacklistPlugin {
    name: String,
    blacklist: Arc<Mutex<Vec<String>>>,
    enabled: bool,
}

impl BlacklistPlugin {
    pub fn new(blacklist: Vec<String>) -> Self {
        Self {
            name: "blacklist".to_string(),
            blacklist: Arc::new(Mutex::new(blacklist)),
            enabled: true,
        }
    }
    
    pub fn add_address(&self, address: impl Into<String>) {
        let mut list = self.blacklist.lock().unwrap();
        list.push(address.into());
    }
    
    pub fn remove_address(&self, address: &str) {
        let mut list = self.blacklist.lock().unwrap();
        list.retain(|a| a != address);
    }
}

impl RulePlugin for BlacklistPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "checktransactionaddress是否在黑名单中"
    }
    
    fn evaluate(&self, context: &TransactionContext) -> RuleResult {
        let list = self.blacklist.lock().unwrap();
        
        if let Some(ref to_addr) = context.to_address {
            if list.contains(to_addr) {
                return RuleResult::threat(
                    ThreatLevel::Critical,
                    format!("目标address {} 在黑名单中", &to_addr[..8.min(to_addr.len())]),
                    1.0,
                    RecommendedAction::Block,
                );
            }
        }
        
        if let Some(ref from_addr) = context.from_address {
            if list.contains(from_addr) {
                return RuleResult::threat(
                    ThreatLevel::Critical,
                    format!("来源address {} 在黑名单中", &from_addr[..8.min(from_addr.len())]),
                    1.0,
                    RecommendedAction::Block,
                );
            }
        }
        
        RuleResult::no_threat()
    }
    
    fn weight(&self) -> f64 {
        10.0 // 黑名单规则具有最高权重
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// 高价值转账插件
pub struct HighValuePlugin {
    name: String,
    threshold: f64,
    enabled: bool,
}

impl HighValuePlugin {
    pub fn new(threshold: f64) -> Self {
        Self {
            name: "high_value".to_string(),
            threshold,
            enabled: true,
        }
    }
    
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold;
    }
}

impl RulePlugin for HighValuePlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "检测高价值转账transaction"
    }
    
    fn evaluate(&self, context: &TransactionContext) -> RuleResult {
        if context.amount > self.threshold {
            let confidence = (context.amount / (self.threshold * 2.0)).min(1.0);
            
            return RuleResult::threat(
                ThreatLevel::High,
                format!("高价值转账: {:.2} (阈值: {:.2})", context.amount, self.threshold),
                confidence,
                RecommendedAction::RequireApproval,
            );
        }
        
        RuleResult::no_threat()
    }
    
    fn weight(&self) -> f64 {
        5.0
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// 尘埃攻击插件
pub struct DustAttackPlugin {
    name: String,
    enabled: bool,
}

impl DustAttackPlugin {
    pub fn new() -> Self {
        Self {
            name: "dust_attack".to_string(),
            enabled: true,
        }
    }
}

impl Default for DustAttackPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RulePlugin for DustAttackPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "检测尘埃攻击（极小金额的转账）"
    }
    
    fn evaluate(&self, context: &TransactionContext) -> RuleResult {
        // 使用 is_dust_amount 特征判断
        if context.features.is_dust_amount > 0.5 {
            return RuleResult::threat(
                ThreatLevel::Medium,
                format!("检测到尘埃攻击: 金额 {:.10}", context.amount),
                0.8,
                RecommendedAction::Warn,
            );
        }
        
        RuleResult::no_threat()
    }
    
    fn weight(&self) -> f64 {
        3.0
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// 新address插件
pub struct NewAddressPlugin {
    name: String,
    enabled: bool,
}

impl NewAddressPlugin {
    pub fn new() -> Self {
        Self {
            name: "new_address".to_string(),
            enabled: true,
        }
    }
}

impl Default for NewAddressPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RulePlugin for NewAddressPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "检测与新address的transaction"
    }
    
    fn evaluate(&self, context: &TransactionContext) -> RuleResult {
        // is_new_address 是 f64 类型 (0.0 或 1.0)
        if context.features.is_new_address > 0.5 && context.amount > 0.1 {
            let threat_level = if context.amount > 10.0 {
                ThreatLevel::High
            } else if context.amount > 1.0 {
                ThreatLevel::Medium
            } else {
                ThreatLevel::Low
            };
            
            return RuleResult::threat(
                threat_level,
                format!("向新address转账 {:.2}", context.amount),
                0.6,
                RecommendedAction::Warn,
            );
        }
        
        RuleResult::no_threat()
    }
    
    fn weight(&self) -> f64 {
        1.5
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_registry() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(BlacklistPlugin::new(vec![]));
        
        assert!(registry.register(plugin.clone()).is_ok());
        assert_eq!(registry.count(), 1);
        
        assert!(registry.register(plugin).is_err()); // 重复注册
        assert_eq!(registry.count(), 1);
        
        assert!(registry.unregister("blacklist").is_ok());
        assert_eq!(registry.count(), 0);
    }
    
    #[test]
    fn test_blacklist_plugin() {
        let plugin = BlacklistPlugin::new(vec!["bad_address".to_string()]);
        
        let features = TransactionFeatures::default();
        let context = TransactionContext::new(features, 1.0)
            .with_addresses(Some("bad_address".to_string()), None);
        
        let result = plugin.evaluate(&context);
        assert!(result.triggered);
        assert_eq!(result.threat_level, ThreatLevel::Critical);
    }
}

