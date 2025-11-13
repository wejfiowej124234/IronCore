// src/tools/generator.rs
//! 配置管理模块
//! 提供配置文件的读取、validate和管理功能

use crate::core::errors::WalletError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 应用级配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 应用基本信息
    pub app: AppConfig,
    /// 模块network级配置
    pub blockchain: BlockchainConfig,
    /// 安全配置
    pub security: SecurityConfig,
    /// 存储配置
    pub storage: StorageConfig,
    /// 监控配置
    pub monitoring: MonitoringConfig,
    /// 国际化配置
    pub i18n: I18nConfig,
}

/// 应用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 应用名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 环境
    pub environment: String,
    /// 测试模式
    pub debug: bool,
    /// 日志级别
    pub log_level: String,
}

/// 模块network级配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    /// 默认network
    pub default_network: String,
    /// network配置列表
    pub networks: HashMap<String, NetworkConfig>,
}

/// network配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// network名称
    pub name: String,
    /// RPC URL
    pub rpc_url: String,
    /// 链 ID
    pub chain_id: u64,
    /// 代币符号
    pub symbol: String,
    /// 区块链浏览器 URL（可选）
    pub explorer_url: Option<String>,
    /// 确认数
    pub confirmations: u64,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 加密算法
    pub encryption_algorithm: String,
    /// 密钥派生算法
    pub kdf_algorithm: String,
    /// 最小Password长度
    pub min_password_length: usize,
    /// 会话超时时间（秒）
    pub session_timeout: u64,
    /// 最大登录尝试次数
    pub max_login_attempts: u32,
    /// 锁定持续时间（秒）
    pub lockout_duration: u64,
    /// 是否启用 2FA
    pub enable_2fa: bool,
    /// 合规check配置
    pub compliance: ComplianceConfig,
}

/// 合规配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    /// 是否启用合规check
    pub enabled: bool,
    /// 受限国家列表
    pub restricted_countries: Vec<String>,
    /// 受限address列表
    pub sanctioned_addresses: Vec<String>,
    /// transaction限额配置
    pub transaction_limits: HashMap<String, f64>,
    /// 是否要求 KYC
    pub require_kyc: bool,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// 数据库类型
    pub database_type: String,
    /// 数据库 URL
    pub database_url: String,
    /// 连接池大小
    pub connection_pool_size: u32,
    /// 缓存大小
    pub cache_size: usize,
    /// 备份间隔（秒）
    pub backup_interval: u64,
    /// 备份保留数量
    pub backup_retention: u32,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 是否启用监控
    pub enabled: bool,
    /// 指标采集间隔（秒）
    pub metrics_interval: u64,
    /// 健康check间隔（秒）
    pub health_check_interval: u64,
    /// 告警阈值配置
    pub alert_thresholds: HashMap<String, f64>,
    /// 日志轮转大小（MB）
    pub log_rotation_size: u64,
    /// 日志保留天数
    pub log_retention_days: u32,
}

/// 国际化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18nConfig {
    /// 默认语言
    pub default_language: String,
    /// 支持的语言列表
    pub supported_languages: Vec<String>,
    /// 翻译文件路径
    pub translation_path: String,
    /// 时区
    pub timezone: String,
}

/// 配置管理器
pub struct ConfigManager {
    config: Config,
    config_path: String,
}

impl Default for ConfigManager {
    /// Creates a new `ConfigManager` with a default configuration file name "config.json".
    fn default() -> Self {
        Self::new("config.json")
    }
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(config_path: impl Into<String>) -> Self {
        Self { config: Config::default(), config_path: config_path.into() }
    }

    /// 加载配置
    pub fn load(&mut self) -> std::result::Result<(), WalletError> {
        if !Path::new(&self.config_path).exists() {
            // 如果配置文件不存在，创建默认配置并保存
            self.config = Config::default();
            self.save()?;
            return Ok(());
        }

        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        self.config = serde_json::from_str(&content)
            .map_err(|e| WalletError::DeserializationError(e.to_string()))?;

        Ok(())
    }

    /// 保存配置
    pub fn save(&self) -> std::result::Result<(), WalletError> {
        let content = serde_json::to_string_pretty(&self.config)
            .map_err(|e| WalletError::SerializationError(e.to_string()))?;

        // 如果父目录存在则创建目录
        if let Some(parent) = Path::new(&self.config_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| WalletError::IoError(e.to_string()))?;
        }

        fs::write(&self.config_path, content)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        Ok(())
    }

    /// fetch只读配置引用
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    /// fetch可变配置引用
    pub fn get_config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 设置配置
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// validate配置
    pub fn validate(&self) -> std::result::Result<(), WalletError> {
        self.config.validate()
    }

    /// 重置为默认配置
    pub fn reset_to_default(&mut self) {
        self.config = Config::default();
    }

    /// fetch配置路径
    pub fn config_path(&self) -> &str {
        &self.config_path
    }
}

impl Default for Config {
    /// Creates a default configuration.
    fn default() -> Self {
        let mut networks = HashMap::new();

        // Ethereum Mainnet
        networks.insert(
            "ethereum_mainnet".to_string(),
            NetworkConfig {
                name: "Ethereum Mainnet".to_string(),
                rpc_url: "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string(),
                chain_id: 1,
                symbol: "ETH".to_string(),
                explorer_url: Some("https://etherscan.io".to_string()),
                confirmations: 12,
            },
        );


        Self {
            app: AppConfig {
                name: "DeFi Hot Wallet".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                environment: "development".to_string(),
                debug: false,
                log_level: "info".to_string(),
            },
            blockchain: BlockchainConfig {
                default_network: "ethereum_mainnet".to_string(),
                networks,
            },
            security: SecurityConfig {
                encryption_algorithm: "AES-256-GCM".to_string(),
                kdf_algorithm: "PBKDF2".to_string(),
                min_password_length: 8,
                session_timeout: 3600, // 1 hour
                max_login_attempts: 5,
                lockout_duration: 900, // 15 minutes
                enable_2fa: true,
                compliance: ComplianceConfig {
                    enabled: true,
                    restricted_countries: vec!["US".to_string(), "CN".to_string()],
                    sanctioned_addresses: vec![],
                    transaction_limits: {
                        let mut limits = HashMap::new();
                        limits.insert("daily".to_string(), 10000.0);
                        limits.insert("monthly".to_string(), 50000.0);
                        limits
                    },
                    require_kyc: false,
                },
            },
            storage: StorageConfig {
                database_type: "SQLite".to_string(),
                database_url: "wallet.db".to_string(),
                connection_pool_size: 10,
                cache_size: 1000,
                backup_interval: 86400, // 1 day
                backup_retention: 30,
            },
            monitoring: MonitoringConfig {
                enabled: true,
                metrics_interval: 60,
                health_check_interval: 300,
                alert_thresholds: {
                    let mut thresholds = HashMap::new();
                    thresholds.insert("cpu_usage".to_string(), 80.0);
                    thresholds.insert("memory_usage".to_string(), 90.0);
                    thresholds
                },
                log_rotation_size: 100, // 100 MB
                log_retention_days: 30,
            },
            i18n: I18nConfig {
                default_language: "en".to_string(),
                supported_languages: vec!["en".to_string(), "zh".to_string(), "es".to_string()],
                translation_path: "translations".to_string(),
                timezone: "UTC".to_string(),
            },
        }
    }
}

impl Config {
    /// validate配置
    pub fn validate(&self) -> std::result::Result<(), WalletError> {
        // validate应用配置
        if self.app.name.is_empty() {
            return Err(WalletError::InvalidInput("App name cannot be empty".to_string()));
        }

        if self.app.version.is_empty() {
            return Err(WalletError::InvalidInput("App version cannot be empty".to_string()));
        }

        // validate区块链模块配置
        if self.blockchain.networks.is_empty() {
            return Err(WalletError::InvalidInput(
                "At least one network must be configured".to_string(),
            ));
        }

        if !self.blockchain.networks.contains_key(&self.blockchain.default_network) {
            return Err(WalletError::InvalidInput(
                "Default network not found in networks".to_string(),
            ));
        }

        // validate安全配置
        if self.security.min_password_length < 8 {
            return Err(WalletError::InvalidInput(
                "Minimum password length must be at least 8".to_string(),
            ));
        }

        // validate存储配置
        if self.storage.database_url.is_empty() {
            return Err(WalletError::InvalidInput("Database URL cannot be empty".to_string()));
        }

        // validate监控配置
        if self.monitoring.enabled && self.monitoring.metrics_interval == 0 {
            return Err(WalletError::InvalidInput("Metrics interval cannot be zero".to_string()));
        }

        // validate国际化配置
        if self.i18n.supported_languages.is_empty() {
            return Err(WalletError::InvalidInput(
                "At least one supported language must be specified".to_string(),
            ));
        }

        if !self.i18n.supported_languages.contains(&self.i18n.default_language) {
            return Err(WalletError::InvalidInput(
                "Default language must be in supported languages".to_string(),
            ));
        }

        Ok(())
    }

    /// fetchnetwork配置
    pub fn get_network(&self, network_name: &str) -> Option<&NetworkConfig> {
        self.blockchain.networks.get(network_name)
    }

    /// fetch默认network配置
    pub fn get_default_network(&self) -> &NetworkConfig {
        self.blockchain
            .networks
            .get(&self.blockchain.default_network)
            .expect("Default network should exist")
    }

    /// checkaddress是否受限
    pub fn is_address_restricted(&self, address: &str) -> bool {
        self.security.compliance.enabled
            && self
                .security
                .compliance
                .sanctioned_addresses
                .iter()
                .any(|restricted| restricted.eq_ignore_ascii_case(address))
    }

    /// check国家是否受限
    pub fn is_country_restricted(&self, country: &str) -> bool {
        self.security.compliance.enabled
            && self
                .security
                .compliance
                .restricted_countries
                .iter()
                .any(|restricted| restricted.eq_ignore_ascii_case(country))
    }

    /// fetch周期transaction限额
    pub fn get_transaction_limit(&self, period: &str) -> Option<f64> {
        if !self.security.compliance.enabled {
            return None;
        }

        self.security.compliance.transaction_limits.get(period).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());

        // Test invalid config
        let mut invalid_config = config.clone();
        invalid_config.app.name = "".to_string();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_config_loading_and_saving() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        // Create and save config
        let manager = ConfigManager::new(config_path.to_str().unwrap());
        manager.save().unwrap();

        // Load config
        let mut new_manager = ConfigManager::new(config_path.to_str().unwrap());
        new_manager.load().unwrap();

        // Verify configs are equal
        assert_eq!(manager.get_config().app.name, new_manager.get_config().app.name);
        assert_eq!(manager.get_config().app.version, new_manager.get_config().app.version);
    }

    #[test]
    fn test_network_config() {
        let config = Config::default();

        let eth_network = config.get_network("ethereum_mainnet").unwrap();
        assert_eq!(eth_network.chain_id, 1);
        assert_eq!(eth_network.symbol, "ETH");

        let default_network = config.get_default_network();
        assert_eq!(default_network.name, "Ethereum Mainnet");
    }

    #[test]
    fn test_compliance_checks() {
        let config = Config::default();

        // Test restricted address
        assert!(!config.is_address_restricted("0x1234567890abcdef"));

        // Test restricted countries - US and CN are restricted by default
        assert!(config.is_country_restricted("US"));
        assert!(config.is_country_restricted("CN"));
        assert!(!config.is_country_restricted("JP")); // Japan is not restricted

        // Test transaction limits
        assert_eq!(config.get_transaction_limit("daily"), Some(10000.0));
        assert_eq!(config.get_transaction_limit("monthly"), Some(50000.0));
        assert_eq!(config.get_transaction_limit("nonexistent"), None);
    }

    #[test]
    fn test_config_modification() {
        let mut config = Config::default();

        // Modify config
        config.app.debug = true;
        config.security.min_password_length = 12;

        // Validate modified config
        assert!(config.validate().is_ok());
        assert!(config.app.debug);
        assert_eq!(config.security.min_password_length, 12);
    }
}
