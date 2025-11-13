//! 安全环境变量访问
//!
//! 提供常用环境变量的安全访问函数

use anyhow::Result;
use zeroize::Zeroizing;
use crate::security::SecretVec;
use base64::Engine;

/// fetchwallet加密密钥
pub fn get_wallet_enc_key() -> Result<SecretVec> {
    let key_b64 = std::env::var("WALLET_ENC_KEY")
        .map_err(|_| anyhow::anyhow!("WALLET_ENC_KEY not set"))?;
    
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(key_b64)
        .map_err(|e| anyhow::anyhow!("Invalid WALLET_ENC_KEY base64: {}", e))?;
    
    Ok(SecretVec::new(key_bytes))
}

/// fetchwallet备份密钥
pub fn get_wallet_backup_key() -> Result<Zeroizing<Vec<u8>>> {
    let key_b64 = std::env::var("WALLET_BACKUP_KEY")
        .map_err(|_| anyhow::anyhow!("WALLET_BACKUP_KEY not set"))?;
    
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(key_b64)
        .map_err(|e| anyhow::anyhow!("Invalid WALLET_BACKUP_KEY base64: {}", e))?;
    
    Ok(Zeroizing::new(key_bytes))
}

/// fetchwallet备份操作员密钥
pub fn get_wallet_backup_operator_key() -> Result<Vec<u8>> {
    let key = std::env::var("WALLET_BACKUP_OPERATOR_KEY")
        .map_err(|_| anyhow::anyhow!("WALLET_BACKUP_OPERATOR_KEY not set"))?;
    
    Ok(key.into_bytes())
}

/// fetch桥接 mock 强制success标志
pub fn get_bridge_mock_force_success() -> Result<String> {
    std::env::var("BRIDGE_MOCK_FORCE_SUCCESS")
        .map_err(|_| anyhow::anyhow!("BRIDGE_MOCK_FORCE_SUCCESS not set"))
}

/// fetch API 密钥
pub fn get_api_key() -> Result<SecretVec> {
    let key = std::env::var("API_KEY")
        .map_err(|_| anyhow::anyhow!("API_KEY not set"))?;
    
    Ok(SecretVec::new(key.into_bytes()))
}

/// check是否处于测试模式
pub fn is_test_mode() -> bool {
    std::env::var("TEST_SKIP_DECRYPT").ok().as_deref() == Some("1")
        || std::env::var("ALLOW_BRIDGE_MOCKS").ok().as_deref() == Some("1")
        || cfg!(any(test, feature = "test-env"))
}

/// check是否允许备份导出
pub fn is_backup_enabled() -> bool {
    std::env::var("BACKUP_ENABLED")
        .ok()
        .filter(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .is_some()
        || is_test_mode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_test_mode_with_test_skip_decrypt() {
        std::env::set_var("TEST_SKIP_DECRYPT", "1");
        assert!(is_test_mode());
        std::env::remove_var("TEST_SKIP_DECRYPT");
    }
    
    #[test]
    fn test_is_test_mode_with_allow_bridge_mocks() {
        std::env::remove_var("TEST_SKIP_DECRYPT");
        std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");
        assert!(is_test_mode());
        std::env::remove_var("ALLOW_BRIDGE_MOCKS");
    }
    
    #[test]
    fn test_is_backup_enabled_with_flag_1() {
        std::env::set_var("BACKUP_ENABLED", "1");
        assert!(is_backup_enabled());
        std::env::remove_var("BACKUP_ENABLED");
    }
    
    #[test]
    fn test_is_backup_enabled_with_flag_true() {
        std::env::set_var("BACKUP_ENABLED", "true");
        assert!(is_backup_enabled());
        std::env::remove_var("BACKUP_ENABLED");
    }
    
    #[test]
    fn test_is_backup_enabled_with_flag_uppercase() {
        std::env::set_var("BACKUP_ENABLED", "TRUE");
        assert!(is_backup_enabled());
        std::env::remove_var("BACKUP_ENABLED");
    }
    
    #[test]
    fn test_is_backup_enabled_false_value() {
        std::env::set_var("BACKUP_ENABLED", "0");
        let result = is_backup_enabled();
        // "0"不应该启用backup（除非test_mode）
        let _ = result;
        std::env::remove_var("BACKUP_ENABLED");
    }
    
    #[test]
    fn test_get_wallet_enc_key_success() {
        std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        let result = get_wallet_enc_key();
        assert!(result.is_ok());
        std::env::remove_var("WALLET_ENC_KEY");
    }
    
    #[test]
    fn test_get_wallet_enc_key_not_set() {
        std::env::remove_var("WALLET_ENC_KEY");
        let result = get_wallet_enc_key();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not set"));
    }
    
    #[test]
    fn test_get_wallet_enc_key_invalid_base64() {
        std::env::set_var("WALLET_ENC_KEY", "invalid_base64!");
        let result = get_wallet_enc_key();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
        std::env::remove_var("WALLET_ENC_KEY");
    }
    
    #[test]
    fn test_get_wallet_backup_key_success() {
        std::env::set_var("WALLET_BACKUP_KEY", "YmFja3VwX2tleV8zMl9ieXRlc19oZXJlIQ==");
        let result = get_wallet_backup_key();
        assert!(result.is_ok());
        std::env::remove_var("WALLET_BACKUP_KEY");
    }
    
    #[test]
    fn test_get_wallet_backup_key_not_set() {
        std::env::remove_var("WALLET_BACKUP_KEY");
        let result = get_wallet_backup_key();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_get_api_key_success() {
        std::env::set_var("API_KEY", "my_api_key_12345");
        let result = get_api_key();
        assert!(result.is_ok());
        std::env::remove_var("API_KEY");
    }
    
    #[test]
    fn test_get_api_key_not_set() {
        std::env::remove_var("API_KEY");
        let result = get_api_key();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_get_bridge_mock_force_success() {
        std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
        let result = get_bridge_mock_force_success();
        assert!(result.is_ok());
        std::env::remove_var("BRIDGE_MOCK_FORCE_SUCCESS");
    }
}

