//! 环境validate
//!
//! 提供环境变量的validate功能

use anyhow::Result;

/// validate环境变量是否完整
pub fn validate_required_env_vars() -> Result<()> {
    let required_in_production = vec![
        "WALLET_ENC_KEY",
        // 其他必需的环境变量
    ];

    // 仅在非测试模式check
    if !cfg!(any(test, feature = "test-env")) {
        for var in required_in_production {
            if std::env::var(var).is_err() {
                return Err(anyhow::anyhow!("Missing required environment variable: {}", var));
            }
        }
    }

    Ok(())
}

/// validate环境变量格式
pub fn validate_env_format(key: &str, value: &str) -> Result<()> {
    match key {
        "WALLET_ENC_KEY" | "WALLET_BACKUP_KEY" => {
            // 应该是 base64 编码
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
                .map_err(|e| anyhow::anyhow!("Invalid base64 for {}: {}", key, e))?;
        }
        _ => {}
    }

    Ok(())
}

/// validate所有环境变量
pub fn validate_all_env_vars() -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    // check必需变量
    if let Err(e) = validate_required_env_vars() {
        warnings.push(e.to_string());
    }

    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_env_format() {
        // Valid base64
        assert!(validate_env_format("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAA==").is_ok());
        
        // Invalid base64
        assert!(validate_env_format("WALLET_ENC_KEY", "not-base64!!!").is_err());
    }
}

