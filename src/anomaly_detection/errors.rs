//! 异常检测error类型
//!
//! 定义统一的error类型，提供更好的error处理和调试信息

use thiserror::Error;

/// 异常检测error类型
#[derive(Debug, Error)]
pub enum AnomalyDetectionError {
    /// 存储相关error
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// 记录未找到
    #[error("Record not found: {0}")]
    NotFound(String),
    
    /// 序列化/反序列化error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// 锁被毒化（线程 panic 时）
    #[error("Lock poisoned")]
    LockPoisoned,
    
    /// 特征提取failed
    #[error("Feature extraction failed: {0}")]
    FeatureExtraction(String),
    
    /// 模型推理failed
    #[error("Model inference failed: {0}")]
    ModelInference(String),
    
    /// 规则评估failed
    #[error("Rule evaluation failed: {0}")]
    RuleEvaluation(String),
    
    /// 插件error
    #[error("Plugin error: {plugin_name} - {message}")]
    Plugin {
        plugin_name: String,
        message: String,
    },
    
    /// 配置error
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// 事件总线error
    #[error("Event bus error: {0}")]
    EventBus(String),
    
    /// 容量已满
    #[error("Storage capacity full: max size {max_size} reached")]
    CapacityFull { max_size: usize },
    
    /// 无效输入
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// 其他error
    #[error("Unknown error: {0}")]
    Other(String),
}

/// 异常检测结果类型
pub type Result<T> = std::result::Result<T, AnomalyDetectionError>;

impl AnomalyDetectionError {
    /// 判断是否为可恢复的error
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::CapacityFull { .. }
                | Self::NotFound(_)
                | Self::InvalidInput(_)
        )
    }
    
    /// 判断是否为严重error
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::LockPoisoned
                | Self::Io(_)
        )
    }
    
    /// fetcherror上下文信息
    pub fn context(&self) -> String {
        match self {
            Self::Plugin { plugin_name, .. } => format!("Plugin: {}", plugin_name),
            Self::CapacityFull { max_size } => format!("Max size: {}", max_size),
            _ => "No additional context".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AnomalyDetectionError::Storage("Connection lost".to_string());
        assert_eq!(err.to_string(), "Storage error: Connection lost");
    }

    #[test]
    fn test_error_is_recoverable() {
        let recoverable = AnomalyDetectionError::NotFound("tx123".to_string());
        assert!(recoverable.is_recoverable());
        
        let critical = AnomalyDetectionError::LockPoisoned;
        assert!(!critical.is_recoverable());
        assert!(critical.is_critical());
    }

    #[test]
    fn test_plugin_error_context() {
        let err = AnomalyDetectionError::Plugin {
            plugin_name: "BlacklistPlugin".to_string(),
            message: "Invalid address".to_string(),
        };
        assert!(err.context().contains("BlacklistPlugin"));
    }
    
    #[test]
    fn test_capacity_error() {
        let err = AnomalyDetectionError::CapacityFull { max_size: 1000 };
        assert!(err.is_recoverable());
        assert!(err.to_string().contains("1000"));
    }
}

