use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// PBKDF2 iteration count
    #[serde(default = "SecurityConfig::default_pbkdf2_iterations")]
    pub pbkdf2_iterations: u32,
    
    /// bcrypt cost factor
    #[serde(default = "SecurityConfig::default_bcrypt_cost")]
    pub bcrypt_cost: u32,
    
    /// Session timeout (seconds)
    #[serde(default = "SecurityConfig::default_session_timeout")]
    pub session_timeout: u64,
    
    /// Maximum sessions per user
    #[serde(default = "SecurityConfig::default_max_sessions_per_user")]
    pub max_sessions_per_user: usize,
    
    /// CSRF token TTL (seconds)
    #[serde(default = "SecurityConfig::default_csrf_ttl")]
    pub csrf_token_ttl: u64,
    
    /// Rate limiter maximum entries
    #[serde(default = "SecurityConfig::default_rate_limiter_max_entries")]
    pub rate_limiter_max_entries: usize,
}

impl SecurityConfig {
    fn default_pbkdf2_iterations() -> u32 { 100_000 }
    fn default_bcrypt_cost() -> u32 { 12 }
    fn default_session_timeout() -> u64 { 3600 }
    fn default_max_sessions_per_user() -> usize { 5 }
    fn default_csrf_ttl() -> u64 { 3600 }
    fn default_rate_limiter_max_entries() -> usize { 10_000 }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            pbkdf2_iterations: Self::default_pbkdf2_iterations(),
            bcrypt_cost: Self::default_bcrypt_cost(),
            session_timeout: Self::default_session_timeout(),
            max_sessions_per_user: Self::default_max_sessions_per_user(),
            csrf_token_ttl: Self::default_csrf_ttl(),
            rate_limiter_max_entries: Self::default_rate_limiter_max_entries(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub database_url: String,
    pub max_connections: Option<u32>,
    pub connection_timeout_seconds: Option<u64>,
}

/// Blockchain network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub name: String,
    pub rpc_url: String,
    pub chain_id: u64,
}

/// Blockchain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub networks: HashMap<String, NetworkConfig>,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        let mut networks = HashMap::with_capacity(4);
        
        networks.insert("eth".to_string(), NetworkConfig {
            name: "Ethereum Mainnet".to_string(),
            rpc_url: "https://eth.llamarpc.com".to_string(),
            chain_id: 1,
        });
        
        networks.insert("sepolia".to_string(), NetworkConfig {
            name: "Sepolia Testnet".to_string(),
            rpc_url: "https://rpc.sepolia.org".to_string(),
            chain_id: 11155111,
        });
        
        Self { networks }
    }
}

/// Derivation path configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationConfig {
    pub path: String,
}

impl Default for DerivationConfig {
    fn default() -> Self {
        Self {
            path: "m/44'/60'/0'/0/0".to_string(),
        }
    }
}

/// wallet配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub storage: StorageConfig,
    pub blockchain: BlockchainConfig,
    pub quantum_safe: bool,
    pub multi_sig_threshold: u8,
    pub derivation: DerivationConfig,
    
    /// 安全配置
    #[serde(default)]
    pub security: SecurityConfig,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig {
                database_url: "sqlite://wallet.db".to_string(),
                max_connections: Some(10),
                connection_timeout_seconds: Some(30),
            },
            blockchain: BlockchainConfig::default(),
            quantum_safe: false,
            multi_sig_threshold: 2,
            derivation: DerivationConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}
