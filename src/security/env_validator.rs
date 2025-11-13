//! 环境变量validate模块
//!
//! 提供生产级环境变量validate，防止配置注入攻击

use anyhow::{Result, bail};
use std::env;
use tracing::{info, warn};

/// 环境变量validate器
pub struct EnvValidator;

impl EnvValidator {
    /// validate数据库URL
    ///
    /// # Security
    /// - 只允许SQLite
    /// - 防止路径遍历
    /// - 防止访问敏感系统文件
    pub fn validate_database_url(url: &str) -> Result<()> {
        // 1. 只允许SQLite协议
        if !url.starts_with("sqlite://") {
            bail!("DATABASE_URL必须使用sqlite://协议（不支持其他数据库）");
        }
        
        // 2. 路径遍历检测
        if url.contains("..") {
            bail!("DATABASE_URL包含路径遍历序列(..)");
        }
        
        // 3. 防止访问敏感系统文件
        let dangerous_paths = ["/etc", "/proc", "/sys", "/dev", "/root", "C:\\Windows", "C:\\Program"];
        for path in &dangerous_paths {
            if url.contains(path) {
                bail!("DATABASE_URL尝试访问敏感系统路径: {}", path);
            }
        }
        
        // 4. 确保是相对路径或明确的数据目录
        if !url.contains("./") && !url.contains("wallets.db") && !url.contains(":memory:") {
            warn!("DATABASE_URL使用非标准路径: {}", url);
        }
        
        info!("✅ DATABASE_URLvalidate通过: {}", url);
        Ok(())
    }
    
    /// validateCORS配置
    ///
    /// # Security
    /// - 拒绝"*"（允许所有源）
    /// - validateURL格式
    /// - 确保使用HTTPS（生产环境）
    pub fn validate_cors_origin(origin: &str) -> Result<()> {
        // 1. 拒绝通配符
        if origin == "*" {
            bail!("CORS_ALLOW_ORIGIN不允许使用'*'（安全风险）");
        }
        
        // 2. validateURL格式
        for url in origin.split(',') {
            let url = url.trim();
            
            if !url.starts_with("http://") && !url.starts_with("https://") {
                bail!("CORS_ALLOW_ORIGIN必须包含协议: {}", url);
            }
            
            // 3. 生产环境建议HTTPS
            if !cfg!(any(test, feature = "test-env")) && url.starts_with("http://") {
                warn!("⚠️  CORS配置使用HTTP（生产环境应使用HTTPS）: {}", url);
            }
        }
        
        info!("✅ CORS_ALLOW_ORIGINvalidate通过: {}", origin);
        Ok(())
    }
    
    /// validateAPI密钥强度
    ///
    /// # Security
    /// - 最小长度32字符
    /// - 拒绝已知弱密钥
    /// - 拒绝测试密钥
    pub fn validate_api_key(key: &str) -> Result<()> {
        // 1. 长度check
        if key.len() < 32 {
            bail!("API_KEY必须至少32字符（当前: {}字符）", key.len());
        }
        
        // 2. 拒绝已知的测试密钥
        const FORBIDDEN_KEYS: &[&str] = &[
            "testnet",
            "test_api_key",
            "example",
            "your-api-key",
            "YOUR_API_KEY",
            "12345",
            "password",
            "secret",
        ];
        
        for forbidden in FORBIDDEN_KEYS {
            if key.to_lowercase().contains(&forbidden.to_lowercase()) {
                bail!("API_KEY包含禁止的测试密钥模式: {}", forbidden);
            }
        }
        
        info!("✅ API_KEYvalidate通过（长度: {}字符）", key.len());
        Ok(())
    }
    
    /// validate加密密钥
    ///
    /// # Security
    /// - 必须是base64编码
    /// - 解码后必须是32字节
    /// - 拒绝全零密钥
    pub fn validate_wallet_enc_key(key_b64: &str) -> Result<()> {
        use base64::Engine as _;
        
        // 1. Base64解码
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(key_b64.trim())
            .map_err(|_| anyhow::anyhow!("WALLET_ENC_KEY不是有效的base64"))?;
        
        // 2. 长度check
        if bytes.len() != 32 {
            bail!("WALLET_ENC_KEY解码后必须是32字节（当前: {}字节）", bytes.len());
        }
        
        // 3. 拒绝全零密钥（仅生产环境）
        #[cfg(not(any(test, feature = "test-env")))]
        {
            if bytes.iter().all(|&b| b == 0) {
                bail!("WALLET_ENC_KEY不能是全零密钥（安全风险）");
            }
        }
        
        info!("✅ WALLET_ENC_KEYvalidate通过（32字节）");
        Ok(())
    }
    
    /// validate所有关键环境变量
    ///
    /// # Returns
    /// Ok(()) if all validations pass
    /// Err(_) with详细error信息
    pub fn validate_all() -> Result<()> {
        info!("startvalidate环境变量配置...");
        
        let mut errors = Vec::new();
        
        // 1. validateWALLET_ENC_KEY（必需）
        match env::var("WALLET_ENC_KEY") {
            Ok(key) => {
                if let Err(e) = Self::validate_wallet_enc_key(&key) {
                    errors.push(format!("WALLET_ENC_KEY: {}", e));
                }
            }
            Err(_) if cfg!(not(any(test, feature = "test-env"))) => {
                errors.push("WALLET_ENC_KEY未设置（生产环境必需）".to_string());
            }
            _ => {}
        }
        
        // 2. validateAPI_KEY（可选但建议）
        if let Ok(key) = env::var("API_KEY") {
            if let Err(e) = Self::validate_api_key(&key) {
                errors.push(format!("API_KEY: {}", e));
            }
        }
        
        // 3. validateDATABASE_URL
        let db_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://./wallets.db".to_string());
        if let Err(e) = Self::validate_database_url(&db_url) {
            errors.push(format!("DATABASE_URL: {}", e));
        }
        
        // 4. validateCORS_ALLOW_ORIGIN
        if let Ok(origin) = env::var("CORS_ALLOW_ORIGIN") {
            if let Err(e) = Self::validate_cors_origin(&origin) {
                errors.push(format!("CORS_ALLOW_ORIGIN: {}", e));
            }
        }
        
        // 5. check是否误用测试环境变量
        #[cfg(not(any(test, feature = "test-env")))]
        {
            if env::var("TEST_SKIP_DECRYPT").is_ok() {
                errors.push("生产环境检测到TEST_SKIP_DECRYPT（仅测试使用）".to_string());
            }
            if env::var("ALLOW_BRIDGE_MOCKS").is_ok() {
                errors.push("生产环境检测到ALLOW_BRIDGE_MOCKS（仅测试使用）".to_string());
            }
        }
        
        // 报告结果
        if !errors.is_empty() {
            warn!("❌ 环境变量validatefailed（{}个error）:", errors.len());
            for (i, error) in errors.iter().enumerate() {
                warn!("  {}. {}", i + 1, error);
            }
            bail!("环境变量validatefailed，请修复上述问题后重启");
        }
        
        info!("✅ 所有环境变量validate通过");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_database_url_valid() {
        assert!(EnvValidator::validate_database_url("sqlite://./wallets.db").is_ok());
        assert!(EnvValidator::validate_database_url("sqlite://:memory:").is_ok());
    }
    
    #[test]
    fn test_validate_database_url_invalid() {
        // 路径遍历
        assert!(EnvValidator::validate_database_url("sqlite://../../../etc/passwd").is_err());
        
        // 敏感路径
        assert!(EnvValidator::validate_database_url("sqlite:///etc/passwd").is_err());
        
        // 非SQLite
        assert!(EnvValidator::validate_database_url("postgresql://localhost/db").is_err());
    }
    
    #[test]
    fn test_validate_api_key() {
        // 太短
        assert!(EnvValidator::validate_api_key("short").is_err());
        
        // 包含禁止模式
        assert!(EnvValidator::validate_api_key("testnet_api_key_xxxxxxxxxxxxxxxxxxxx").is_err());
        
        // 有效密钥
        assert!(EnvValidator::validate_api_key("a".repeat(32).as_str()).is_ok());
    }
    
    #[test]
    fn test_validate_cors_wildcard() {
        // 拒绝通配符
        assert!(EnvValidator::validate_cors_origin("*").is_err());
    }
    
    #[test]
    fn test_validate_cors_valid() {
        assert!(EnvValidator::validate_cors_origin("https://example.com").is_ok());
        assert!(EnvValidator::validate_cors_origin("https://a.com,https://b.com").is_ok());
    }
}

