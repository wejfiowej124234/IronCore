//! error消息脱敏模块
//!
//! 防止error消息泄漏敏感信息

use regex::Regex;
use once_cell::sync::Lazy;

/// 敏感信息的正则表达式模式
static SENSITIVE_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        // Private key（十六进制，32字节=64字符）
        (Regex::new(r"0x[0-9a-fA-F]{64}").unwrap(), "[REDACTED_PRIVATE_KEY]"),
        // mnemonic（12/24个单词）
        (Regex::new(r"\b([a-z]{3,8}\s+){11,23}[a-z]{3,8}\b").unwrap(), "[REDACTED_MNEMONIC]"),
        // API密钥（各种格式）
        (Regex::new(r"(?i)api[_-]?key['\x22]?\s*[:=]\s*['\x22]?[a-zA-Z0-9_-]{20,}").unwrap(), "api_key=[REDACTED]"),
        // JWT Token
        (Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").unwrap(), "[REDACTED_JWT]"),
        // Password（常见格式）
        (Regex::new(r"(?i)password['\x22]?\s*[:=]\s*['\x22]?[^\s'\x22]{6,}").unwrap(), "password=[REDACTED]"),
        // Emailaddress
        (Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(), "[REDACTED_EMAIL]"),
        // IPaddress（完全隐藏）
        (Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(), "xxx.xxx.xxx.xxx"),
        // 文件路径（Windows/Linux）
        (Regex::new(r"(?i)[a-z]:\\[^\s]+|/home/[^\s]+|/root/[^\s]+").unwrap(), "[REDACTED_PATH]"),
        // 数据库连接字符串
        (Regex::new(r"(?i)(?:postgres|mysql|mongodb)://[^\s]+").unwrap(), "[DATABASE_URL_REDACTED]"),
    ]
});

/// 脱敏error消息
///
/// # Arguments
/// * `message` - 原始error消息
///
/// # Returns
/// 脱敏后的安全error消息
pub fn sanitize_error_message(message: &str) -> String {
    let mut sanitized = message.to_string();
    
    for (pattern, replacement) in SENSITIVE_PATTERNS.iter() {
        sanitized = pattern.replace_all(&sanitized, *replacement).to_string();
    }
    
    sanitized
}

/// 生成user友好的error消息（不泄漏技术细节）
///
/// # Arguments
/// * `error_type` - error类型（用于内部日志）
///
/// # Returns
/// user友好的通用error消息
/// 生成user友好的error消息（不泄漏技术细节）
///
/// # Arguments
/// * `error_type` - error类型（用于内部日志）
///
/// # Returns
/// user友好的通用error消息
pub fn user_friendly_error(error_type: &str) -> String {
    match error_type {
        "DecryptionError" => "无法解密数据，请checkPassword是否正确".to_string(),
        "InvalidPrivateKey" => "Private key格式无效".to_string(),
        "NetworkError" => "network连接failed，请稍后重试".to_string(),
        "DatabaseError" => "数据库操作failed".to_string(),
        "ValidationError" => "输入validatefailed".to_string(),
        "InsufficientFunds" => "balance不足".to_string(),
        "Unauthorized" => "未授权访问".to_string(),
        "TransactionFailed" => "transactionfailed，请稍后重试".to_string(),
        "InvalidInput" => "输入数据无效".to_string(),
        "InvalidAddress" => "address格式无效".to_string(),
        "InvalidAmount" => "金额格式无效".to_string(),
        "WalletCreationFailed" => "wallet创建failed".to_string(),
        "WalletNotFound" => "wallet不存在".to_string(),
        "BridgeFailed" => "跨链桥接failed".to_string(),
        "BalanceQueryFailed" => "balancequeryfailed".to_string(),
        _ => "操作failed，请稍后重试".to_string(),
    }
}

/// 日志专用脱敏（保留更多调试信息，但隐藏敏感数据）
pub fn sanitize_for_logging(message: &str) -> String {
    let mut sanitized = message.to_string();
    
    // 只脱敏最敏感的信息（用于内部日志）
    let critical_patterns = vec![
        (Regex::new(r"0x[0-9a-fA-F]{64}").unwrap(), "0x[REDACTED_64_CHARS]"),
        (Regex::new(r"\b([a-z]{3,8}\s+){11,23}[a-z]{3,8}\b").unwrap(), "[MNEMONIC_PHRASE]"),
        (Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").unwrap(), "eyJ...[JWT]"),
    ];
    
    for (pattern, replacement) in critical_patterns {
        sanitized = pattern.replace_all(&sanitized, replacement).to_string();
    }
    
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_private_key() {
        let msg = "Error: invalid private key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let sanitized = sanitize_error_message(msg);
        assert!(sanitized.contains("[REDACTED_PRIVATE_KEY]"));
        assert!(!sanitized.contains("1234567890abcdef"));
    }
    
    #[test]
    fn test_sanitize_mnemonic() {
        let msg = "Mnemonic: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let sanitized = sanitize_error_message(msg);
        assert!(sanitized.contains("[REDACTED_MNEMONIC]"));
        assert!(!sanitized.contains("abandon about"));
    }
    
    #[test]
    fn test_sanitize_email() {
        let msg = "User test@example.com failed authentication";
        let sanitized = sanitize_error_message(msg);
        assert!(sanitized.contains("[REDACTED_EMAIL]"));
        assert!(!sanitized.contains("test@example.com"));
    }
    
    #[test]
    fn test_sanitize_jwt() {
        let msg = "Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let sanitized = sanitize_error_message(msg);
        assert!(sanitized.contains("[REDACTED_JWT]"));
        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }
    
    #[test]
    fn test_user_friendly_error() {
        assert_eq!(user_friendly_error("DecryptionError"), "无法解密数据，请checkPassword是否正确");
        assert_eq!(user_friendly_error("NetworkError"), "network连接failed，请稍后重试");
        assert_eq!(user_friendly_error("UnknownError"), "操作failed，请稍后重试");
    }
}

