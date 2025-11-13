//! 存储抽象层
//!
//! 提供灵活的存储后端接口，支持内存、文件和数据库存储

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::anomaly_detection::{AnomalyResult, errors::{AnomalyDetectionError, Result}};

/// 检测记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRecord {
    /// Transaction hash
    pub transaction_hash: String,
    
    /// 检测结果
    pub result: AnomalyResult,
    
    /// 检测时间
    pub timestamp: DateTime<Utc>,
    
    /// 检测耗时（毫秒）
    pub duration_ms: u64,
    
    /// 区块链类型
    pub blockchain: String,
    
    /// 额外元数据
    pub metadata: HashMap<String, String>,
}

/// 存储后端 trait
pub trait StorageBackend: Send + Sync {
    /// 保存检测记录
    fn save_record(&self, record: DetectionRecord) -> Result<()>;
    
    /// fetch检测记录
    fn get_record(&self, transaction_hash: &str) -> Result<Option<DetectionRecord>>;
    
    /// query记录
    fn query_records(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<DetectionRecord>>;
    
    /// Delete记录
    fn delete_record(&self, transaction_hash: &str) -> Result<()>;
    
    /// 清空所有记录
    fn clear(&self) -> Result<()>;
    
    /// fetch记录数量
    fn count(&self) -> Result<usize>;
}

/// 内存存储后端（优化版 - 使用 RwLock）
pub struct MemoryStorage {
    records: Arc<RwLock<HashMap<String, DetectionRecord>>>,
    max_size: usize,
}

impl MemoryStorage {
    /// 创建新的内存存储
    pub fn new(max_size: usize) -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }
    
    /// 清理旧记录（当达到最大容量时）
    fn evict_old_records(&self, records: &mut HashMap<String, DetectionRecord>) -> Result<()> {
        if records.len() >= self.max_size {
            // 找到最旧的记录并Delete
            if let Some(oldest_key) = records
                .iter()
                .min_by_key(|(_, record)| record.timestamp)
                .map(|(key, _)| key.clone())
            {
                records.remove(&oldest_key);
                Ok(())
            } else {
                Err(AnomalyDetectionError::Storage(
                    "Failed to evict old records".to_string()
                ))
            }
        } else {
            Ok(())
        }
    }
}

impl StorageBackend for MemoryStorage {
    fn save_record(&self, record: DetectionRecord) -> Result<()> {
        let mut records = self.records.write()
            .map_err(|_| AnomalyDetectionError::LockPoisoned)?;
        
        // check容量并清理旧记录
        self.evict_old_records(&mut records)?;
        
        records.insert(record.transaction_hash.clone(), record);
        Ok(())
    }
    
    fn get_record(&self, transaction_hash: &str) -> Result<Option<DetectionRecord>> {
        let records = self.records.read()
            .map_err(|_| AnomalyDetectionError::LockPoisoned)?;
        Ok(records.get(transaction_hash).cloned())
    }
    
    fn query_records(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<DetectionRecord>> {
        let records = self.records.read()
            .map_err(|_| AnomalyDetectionError::LockPoisoned)?;
        
        let mut filtered: Vec<DetectionRecord> = records
            .values()
            .filter(|record| {
                if let Some(start) = start_time {
                    if record.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = end_time {
                    if record.timestamp > end {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        
        // 按时间戳排序（最新的在前）
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // 限制数量
        filtered.truncate(limit);
        
        Ok(filtered)
    }
    
    fn delete_record(&self, transaction_hash: &str) -> Result<()> {
        let mut records = self.records.write()
            .map_err(|_| AnomalyDetectionError::LockPoisoned)?;
        records.remove(transaction_hash);
        Ok(())
    }
    
    fn clear(&self) -> Result<()> {
        let mut records = self.records.write()
            .map_err(|_| AnomalyDetectionError::LockPoisoned)?;
        records.clear();
        Ok(())
    }
    
    fn count(&self) -> Result<usize> {
        let records = self.records.read()
            .map_err(|_| AnomalyDetectionError::LockPoisoned)?;
        Ok(records.len())
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// address历史存储
pub struct AddressHistory {
    history: Arc<RwLock<HashMap<String, AddressInfo>>>,
}

impl AddressHistory {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 记录address活动
    pub fn record_activity(&self, address: &str, amount: f64, is_from: bool) {
        let Ok(mut history) = self.history.write() else {
            return;
        };
        
        let info = history.entry(address.to_string()).or_insert_with(|| AddressInfo {
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            transaction_count: 0,
            total_sent: 0.0,
            total_received: 0.0,
        });
        
        info.last_seen = Utc::now();
        info.transaction_count += 1;
        
        if is_from {
            info.total_sent += amount;
        } else {
            info.total_received += amount;
        }
    }
    
    /// fetchaddress信息
    pub fn get_info(&self, address: &str) -> Option<AddressInfo> {
        let history = self.history.read().ok()?;
        history.get(address).cloned()
    }
    
    /// checkaddress是否为新address
    pub fn is_new_address(&self, address: &str, threshold_secs: u64) -> bool {
        let Ok(history) = self.history.read() else {
            return false;
        };
        
        if let Some(info) = history.get(address) {
            let age = Utc::now().signed_duration_since(info.first_seen);
            age.num_seconds() < threshold_secs as i64
        } else {
            true // 未见过的address视为新address
        }
    }
    
    /// 清理旧数据
    pub fn cleanup_old_entries(&self, retention_days: u64) {
        let Ok(mut history) = self.history.write() else {
            return;
        };
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        
        history.retain(|_, info| info.last_seen > cutoff);
    }
}

impl Default for AddressHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// address信息
#[derive(Debug, Clone)]
pub struct AddressInfo {
    /// 首次见到时间
    pub first_seen: DateTime<Utc>,
    
    /// 最后见到时间
    pub last_seen: DateTime<Utc>,
    
    /// transaction次数
    pub transaction_count: u64,
    
    /// 总发送金额
    pub total_sent: f64,
    
    /// 总接收金额
    pub total_received: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomaly_detection::ThreatLevel;
    
    #[test]
    fn test_memory_storage() {
        let storage = MemoryStorage::new(10);
        
        let record = DetectionRecord {
            transaction_hash: "test_hash".to_string(),
            result: AnomalyResult {
                is_anomalous: false,
                score: 0.1,
                threat_level: ThreatLevel::None,
                reason: "test".to_string(),
                key_factors: vec![],
            },
            timestamp: Utc::now(),
            duration_ms: 100,
            blockchain: "polygon".to_string(),
            metadata: HashMap::new(),
        };
        
        assert!(storage.save_record(record.clone()).is_ok());
        assert_eq!(storage.count().unwrap(), 1);
        
        let retrieved = storage.get_record("test_hash").unwrap();
        assert!(retrieved.is_some());
        
        assert!(storage.delete_record("test_hash").is_ok());
        assert_eq!(storage.count().unwrap(), 0);
    }
    
    #[test]
    fn test_address_history() {
        let history = AddressHistory::new();
        
        history.record_activity("test_address", 1.0, true);
        
        let info = history.get_info("test_address");
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert_eq!(info.transaction_count, 1);
        assert_eq!(info.total_sent, 1.0);
        assert_eq!(info.total_received, 0.0);
    }
    
    #[test]
    fn test_query_records() {
        let storage = MemoryStorage::new(100);
        
        for i in 0..5 {
            let record = DetectionRecord {
                transaction_hash: format!("hash_{}", i),
                result: AnomalyResult {
                    is_anomalous: false,
                    score: 0.1,
                    threat_level: ThreatLevel::None,
                    reason: "test".to_string(),
                    key_factors: vec![],
                },
                timestamp: Utc::now(),
                duration_ms: 100,
                blockchain: "polygon".to_string(),
                metadata: HashMap::new(),
            };
            storage.save_record(record).unwrap();
        }
        
        let records = storage.query_records(None, None, 10).unwrap();
        assert_eq!(records.len(), 5);
        
        let records = storage.query_records(None, None, 3).unwrap();
        assert_eq!(records.len(), 3);
    }
}

