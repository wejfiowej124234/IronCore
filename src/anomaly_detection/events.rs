//! 事件系统模块
//!
//! 提供异常检测事件的发布-订阅机制

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::anomaly_detection::{AnomalyResult, ThreatLevel};

/// 异常检测事件
#[derive(Debug, Clone)]
pub enum AnomalyEvent {
    /// 检测start
    DetectionStarted {
        transaction_hash: String,
        timestamp: DateTime<Utc>,
    },
    
    /// 检测completed
    DetectionCompleted {
        transaction_hash: String,
        result: AnomalyResult,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    
    /// 规则触发
    RuleTriggered {
        transaction_hash: String,
        rule_name: String,
        threat_level: ThreatLevel,
        details: String,
        timestamp: DateTime<Utc>,
    },
    
    /// 模型预测
    ModelPrediction {
        transaction_hash: String,
        score: f64,
        features: Vec<f64>,
        timestamp: DateTime<Utc>,
    },
    
    /// transaction被阻止
    TransactionBlocked {
        transaction_hash: String,
        reason: String,
        threat_level: ThreatLevel,
        timestamp: DateTime<Utc>,
    },
    
    /// Warning发出
    WarningIssued {
        transaction_hash: String,
        message: String,
        threat_level: ThreatLevel,
        timestamp: DateTime<Utc>,
    },
    
    /// 检测error
    DetectionError {
        transaction_hash: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    
    /// 配置更新
    ConfigurationUpdated {
        changes: HashMap<String, String>,
        timestamp: DateTime<Utc>,
    },
    
    /// 统计更新
    StatisticsUpdated {
        total_detections: u64,
        anomalies_detected: u64,
        false_positives: u64,
        timestamp: DateTime<Utc>,
    },
}

impl AnomalyEvent {
    /// fetch事件类型名称
    pub fn event_type(&self) -> &'static str {
        match self {
            AnomalyEvent::DetectionStarted { .. } => "detection_started",
            AnomalyEvent::DetectionCompleted { .. } => "detection_completed",
            AnomalyEvent::RuleTriggered { .. } => "rule_triggered",
            AnomalyEvent::ModelPrediction { .. } => "model_prediction",
            AnomalyEvent::TransactionBlocked { .. } => "transaction_blocked",
            AnomalyEvent::WarningIssued { .. } => "warning_issued",
            AnomalyEvent::DetectionError { .. } => "detection_error",
            AnomalyEvent::ConfigurationUpdated { .. } => "configuration_updated",
            AnomalyEvent::StatisticsUpdated { .. } => "statistics_updated",
        }
    }
    
    /// fetch事件时间戳
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            AnomalyEvent::DetectionStarted { timestamp, .. } => *timestamp,
            AnomalyEvent::DetectionCompleted { timestamp, .. } => *timestamp,
            AnomalyEvent::RuleTriggered { timestamp, .. } => *timestamp,
            AnomalyEvent::ModelPrediction { timestamp, .. } => *timestamp,
            AnomalyEvent::TransactionBlocked { timestamp, .. } => *timestamp,
            AnomalyEvent::WarningIssued { timestamp, .. } => *timestamp,
            AnomalyEvent::DetectionError { timestamp, .. } => *timestamp,
            AnomalyEvent::ConfigurationUpdated { timestamp, .. } => *timestamp,
            AnomalyEvent::StatisticsUpdated { timestamp, .. } => *timestamp,
        }
    }
}

/// 事件订阅者 trait
pub trait EventSubscriber: Send + Sync {
    /// 处理事件
    fn on_event(&self, event: &AnomalyEvent);
    
    /// fetch订阅者名称
    fn name(&self) -> &str;
    
    /// fetch感兴趣的事件类型
    fn interested_events(&self) -> Vec<&'static str>;
}

/// 事件总线
pub struct EventBus {
    subscribers: Arc<Mutex<Vec<Arc<dyn EventSubscriber>>>>,
    event_buffer: Arc<Mutex<Vec<AnomalyEvent>>>,
    buffer_size: usize,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new(buffer_size: usize) -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            event_buffer: Arc::new(Mutex::new(Vec::with_capacity(buffer_size))),
            buffer_size,
        }
    }
    
    /// 订阅事件
    pub fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(subscriber);
    }
    
    /// Cancel订阅
    pub fn unsubscribe(&self, subscriber_name: &str) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.retain(|s| s.name() != subscriber_name);
    }
    
    /// 发布事件
    pub fn publish(&self, event: AnomalyEvent) {
        // 添加到缓冲区
        {
            let mut buffer = self.event_buffer.lock().unwrap();
            if buffer.len() >= self.buffer_size {
                buffer.remove(0);
            }
            buffer.push(event.clone());
        }
        
        // 通知所有订阅者
        let subs = self.subscribers.lock().unwrap();
        for subscriber in subs.iter() {
            let interested = subscriber.interested_events();
            if interested.is_empty() || interested.contains(&event.event_type()) {
                subscriber.on_event(&event);
            }
        }
    }
    
    /// fetch最近的事件
    pub fn get_recent_events(&self, count: usize) -> Vec<AnomalyEvent> {
        let buffer = self.event_buffer.lock().unwrap();
        let start = buffer.len().saturating_sub(count);
        buffer[start..].to_vec()
    }
    
    /// 清空事件缓冲区
    pub fn clear_buffer(&self) {
        let mut buffer = self.event_buffer.lock().unwrap();
        buffer.clear();
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// 日志订阅者 - 将事件记录到日志
pub struct LoggingSubscriber {
    name: String,
}

impl LoggingSubscriber {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }
}

impl EventSubscriber for LoggingSubscriber {
    fn on_event(&self, event: &AnomalyEvent) {
        use tracing::{info, warn, error};
        use crate::security::redaction::redact_body;
        
        match event {
            AnomalyEvent::DetectionCompleted { transaction_hash, result, duration_ms, .. } => {
                info!(
                    transaction_hash = &transaction_hash[..8.min(transaction_hash.len())],
                    is_anomalous = result.is_anomalous,
                    score = %format!("{:.2}", result.score),
                    duration_ms = duration_ms,
                    "Anomaly detection completed"
                );
            }
            AnomalyEvent::TransactionBlocked { transaction_hash, reason, threat_level, .. } => {
                warn!(
                    transaction_hash = &transaction_hash[..8.min(transaction_hash.len())],
                    threat_level = ?threat_level,
                    reason = %redact_body(reason),  // 脱敏原因，防止泄漏敏感信息
                    "Transaction blocked by anomaly detection"
                );
            }
            AnomalyEvent::RuleTriggered { transaction_hash, rule_name, threat_level, details, .. } => {
                warn!(
                    transaction_hash = &transaction_hash[..8.min(transaction_hash.len())],
                    rule_name = rule_name,
                    threat_level = ?threat_level,
                    details = %redact_body(details),  // 脱敏详情
                    "Anomaly detection rule triggered"
                );
            }
            AnomalyEvent::DetectionError { transaction_hash, error, .. } => {
                error!(
                    transaction_hash = &transaction_hash[..8.min(transaction_hash.len())],
                    error = %redact_body(error),  // 脱敏error信息，防止泄漏内部路径
                    "Anomaly detection failed"
                );
            }
            _ => {}
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn interested_events(&self) -> Vec<&'static str> {
        vec![] // 订阅所有事件
    }
}

/// 统计订阅者 - 收集统计信息
pub struct StatisticsSubscriber {
    name: String,
    stats: Arc<Mutex<DetectionStatistics>>,
}

impl StatisticsSubscriber {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stats: Arc::new(Mutex::new(DetectionStatistics::default())),
        }
    }
    
    pub fn get_statistics(&self) -> DetectionStatistics {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }
    
    pub fn reset_statistics(&self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = DetectionStatistics::default();
    }
}

impl EventSubscriber for StatisticsSubscriber {
    fn on_event(&self, event: &AnomalyEvent) {
        let mut stats = self.stats.lock().unwrap();
        
        match event {
            AnomalyEvent::DetectionCompleted { result, duration_ms, .. } => {
                stats.total_detections += 1;
                stats.total_duration_ms += duration_ms;
                
                if result.is_anomalous {
                    stats.anomalies_detected += 1;
                    
                    match result.threat_level {
                        ThreatLevel::None => {}
                        ThreatLevel::Low => stats.low_threats += 1,
                        ThreatLevel::Medium => stats.medium_threats += 1,
                        ThreatLevel::High => stats.high_threats += 1,
                        ThreatLevel::Critical => stats.critical_threats += 1,
                    }
                }
            }
            AnomalyEvent::TransactionBlocked { .. } => {
                stats.transactions_blocked += 1;
            }
            AnomalyEvent::DetectionError { .. } => {
                stats.detection_errors += 1;
            }
            _ => {}
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn interested_events(&self) -> Vec<&'static str> {
        vec![
            "detection_completed",
            "transaction_blocked",
            "detection_error",
        ]
    }
}

/// 检测统计信息
#[derive(Debug, Clone, Default)]
pub struct DetectionStatistics {
    pub total_detections: u64,
    pub anomalies_detected: u64,
    pub transactions_blocked: u64,
    pub detection_errors: u64,
    pub total_duration_ms: u64,
    pub low_threats: u64,
    pub medium_threats: u64,
    pub high_threats: u64,
    pub critical_threats: u64,
}

impl DetectionStatistics {
    pub fn average_duration_ms(&self) -> f64 {
        if self.total_detections == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.total_detections as f64
        }
    }
    
    pub fn anomaly_rate(&self) -> f64 {
        if self.total_detections == 0 {
            0.0
        } else {
            self.anomalies_detected as f64 / self.total_detections as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_bus() {
        let bus = EventBus::new(10);
        let subscriber = Arc::new(LoggingSubscriber::new("test"));
        
        bus.subscribe(subscriber);
        
        let event = AnomalyEvent::DetectionStarted {
            transaction_hash: "test_hash".to_string(),
            timestamp: Utc::now(),
        };
        
        bus.publish(event);
        
        let recent = bus.get_recent_events(5);
        assert_eq!(recent.len(), 1);
    }
    
    #[test]
    fn test_statistics_subscriber() {
        let subscriber = StatisticsSubscriber::new("stats");
        
        let event = AnomalyEvent::DetectionCompleted {
            transaction_hash: "test".to_string(),
            result: AnomalyResult {
                is_anomalous: true,
                score: 0.8,
                threat_level: ThreatLevel::High,
                reason: "test".to_string(),
                key_factors: vec![],
            },
            duration_ms: 100,
            timestamp: Utc::now(),
        };
        
        subscriber.on_event(&event);
        
        let stats = subscriber.get_statistics();
        assert_eq!(stats.total_detections, 1);
        assert_eq!(stats.anomalies_detected, 1);
        assert_eq!(stats.high_threats, 1);
    }

    #[test]
    fn test_event_bus_publish_subscribe() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let bus = EventBus::new(100);
        
        // 创建一个计数订阅者
        struct CountingSubscriber {
            count: Arc<AtomicUsize>,
        }
        
        impl EventSubscriber for CountingSubscriber {
            fn name(&self) -> &str {
                "CountingSubscriber"
            }
            
            fn interested_events(&self) -> Vec<&'static str> {
                vec![]  // 空vec表示接收所有事件
            }
            
            fn on_event(&self, _event: &AnomalyEvent) {
                self.count.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        let counter = Arc::new(AtomicUsize::new(0));
        let subscriber = Arc::new(CountingSubscriber {
            count: counter.clone(),
        });
        
        // 订阅事件
        bus.subscribe(subscriber);
        
        // 发布多个事件
        for i in 0..5 {
            bus.publish(AnomalyEvent::DetectionStarted {
                transaction_hash: format!("tx{}", i),
                timestamp: Utc::now(),
            });
        }
        
        // 等待事件处理
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // validate订阅者收到了所有事件
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_multiple_subscribers() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let bus = EventBus::new(100);
        
        struct CountingSubscriber {
            count: Arc<AtomicUsize>,
        }
        
        impl EventSubscriber for CountingSubscriber {
            fn name(&self) -> &str {
                "CountingSubscriber"
            }
            
            fn interested_events(&self) -> Vec<&'static str> {
                vec![]  // 空vec表示接收所有事件
            }
            
            fn on_event(&self, _event: &AnomalyEvent) {
                self.count.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));
        
        // 添加两个订阅者
        bus.subscribe(Arc::new(CountingSubscriber { count: counter1.clone() }));
        bus.subscribe(Arc::new(CountingSubscriber { count: counter2.clone() }));
        
        // 发布事件
        bus.publish(AnomalyEvent::DetectionStarted {
            transaction_hash: "tx1".to_string(),
            timestamp: Utc::now(),
        });
        
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // 两个订阅者都应该收到事件
        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_all_variants() {
        use crate::anomaly_detection::AnomalyResult;
        
        let events = vec![
            AnomalyEvent::DetectionStarted {
                transaction_hash: "tx1".to_string(),
                timestamp: Utc::now(),
            },
            AnomalyEvent::DetectionCompleted {
                transaction_hash: "tx1".to_string(),
                result: AnomalyResult::normal(),
                duration_ms: 10,
                timestamp: Utc::now(),
            },
            AnomalyEvent::RuleTriggered {
                transaction_hash: "tx2".to_string(),
                rule_name: "blacklist".to_string(),
                threat_level: ThreatLevel::High,
                details: "Address blacklisted".to_string(),
                timestamp: Utc::now(),
            },
            AnomalyEvent::TransactionBlocked {
                transaction_hash: "tx3".to_string(),
                reason: "High risk".to_string(),
                threat_level: ThreatLevel::Critical,
                timestamp: Utc::now(),
            },
            AnomalyEvent::WarningIssued {
                transaction_hash: "tx4".to_string(),
                message: "Suspicious pattern".to_string(),
                threat_level: ThreatLevel::Medium,
                timestamp: Utc::now(),
            },
        ];
        
        // 确保所有事件类型都可以创建和克隆
        for event in events {
            let _cloned = event.clone();
        }
    }

    #[test]
    fn test_event_bus_empty_subscribers() {
        let bus = EventBus::new(10);
        
        // 没有订阅者时发布事件不应该 panic
        bus.publish(AnomalyEvent::DetectionStarted {
            transaction_hash: "tx1".to_string(),
            timestamp: Utc::now(),
        });
    }

    #[test]
    fn test_get_recent_events() {
        let bus = EventBus::new(100);
        
        // 发布10个事件
        for i in 0..10 {
            bus.publish(AnomalyEvent::DetectionStarted {
                transaction_hash: format!("tx{}", i),
                timestamp: Utc::now(),
            });
        }
        
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // fetch最近5个事件
        let recent = bus.get_recent_events(5);
        assert!(recent.len() <= 5);
    }

    #[test]
    fn test_statistics_subscriber_accuracy() {
        let subscriber = StatisticsSubscriber::new("test");
        
        // 发送正常transaction
        for _ in 0..10 {
            subscriber.on_event(&AnomalyEvent::DetectionCompleted {
                transaction_hash: "tx".to_string(),
                result: crate::anomaly_detection::AnomalyResult::normal(),
                duration_ms: 5,
                timestamp: Utc::now(),
            });
        }
        
        // 发送异常transaction
        for level in [ThreatLevel::Low, ThreatLevel::Medium, ThreatLevel::High, ThreatLevel::Critical] {
            subscriber.on_event(&AnomalyEvent::DetectionCompleted {
                transaction_hash: "tx_anomaly".to_string(),
                result: crate::anomaly_detection::AnomalyResult::anomalous(
                    0.9,
                    level,
                    "Test".to_string(),
                ),
                duration_ms: 10,
                timestamp: Utc::now(),
            });
        }
        
        let stats = subscriber.get_statistics();
        assert_eq!(stats.total_detections, 14);
        assert_eq!(stats.anomalies_detected, 4);
        assert_eq!(stats.low_threats, 1);
        assert_eq!(stats.medium_threats, 1);
        assert_eq!(stats.high_threats, 1);
        assert_eq!(stats.critical_threats, 1);
    }
}

