//! Password管理服务

use crate::auth::{config::PasswordConfig, errors::AuthError};
use tracing::info;

/// Password管理服务
pub struct PasswordService {
    config: PasswordConfig,
}

impl PasswordService {
    /// 创建新的Password服务
    pub fn new(config: PasswordConfig) -> Self {
        Self { config }
    }
    
    /// validatePassword强度
    pub fn validate_strength(&self, password: &str) -> Result<(), AuthError> {
        // 1. 长度check
        if password.len() < self.config.min_length {
            return Err(AuthError::WeakPassword(
                format!("Password长度至少{}位", self.config.min_length)
            ));
        }
        
        // 2. 数字check
        if self.config.require_digit && !password.chars().any(|c| c.is_numeric()) {
            return Err(AuthError::WeakPassword(
                "Password必须包含至少一个数字".to_string()
            ));
        }
        
        // 3. 字母check
        if self.config.require_letter && !password.chars().any(|c| c.is_alphabetic()) {
            return Err(AuthError::WeakPassword(
                "Password必须包含至少一个字母".to_string()
            ));
        }
        
        // 4. 特殊字符check（可选）
        if self.config.require_special {
            let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
            if !password.chars().any(|c| special_chars.contains(c)) {
                return Err(AuthError::WeakPassword(
                    "Password必须包含至少一个特殊字符".to_string()
                ));
            }
        }
        
        info!("Password强度validate通过");
        Ok(())
    }
    
    /// 哈希Password（使用bcrypt）
    pub fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        bcrypt::hash(password, self.config.bcrypt_cost)
            .map_err(|e| AuthError::InternalError(format!("Password哈希failed: {}", e)))
    }
    
    /// validatePassword（使用bcrypt）
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        bcrypt::verify(password, hash)
            .map_err(|e| AuthError::InternalError(format!("Passwordvalidatefailed: {}", e)))
    }
    
    /// 计算Password强度分数（0-100）
    pub fn calculate_strength_score(&self, password: &str) -> u8 {
        let mut score = 0u8;
        
        // 长度加分
        if password.len() >= 8 { score += 25; }
        // 适度奖励更长但未到12位的Password，提升整体评分区分度
        if password.len() >= 10 { score += 5; }
        if password.len() >= 12 { score += 15; }
        if password.len() >= 16 { score += 10; }
        
        // 字符类型加分
        if password.chars().any(|c| c.is_lowercase()) { score += 10; }
        if password.chars().any(|c| c.is_uppercase()) { score += 10; }
        if password.chars().any(|c| c.is_numeric()) { score += 15; }
        
        // 特殊字符加分
        let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        if password.chars().any(|c| special_chars.contains(c)) { score += 15; }
        
        score
    }
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new(PasswordConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        let service = PasswordService::default();
        
        // 太短
        assert!(service.validate_strength("123").is_err());
        
        // 缺少数字
        assert!(service.validate_strength("abcdefgh").is_err());
        
        // 缺少字母
        assert!(service.validate_strength("12345678").is_err());
        
        // 有效Password
        assert!(service.validate_strength("Test1234").is_ok());
    }

    #[test]
    fn test_password_hashing() {
        let service = PasswordService::default();
        
        let password = "Test1234";
        let hash = service.hash_password(password).unwrap();
        
        // validatesuccess
        assert!(service.verify_password(password, &hash).unwrap());
        
        // validatefailed
        assert!(!service.verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn test_strength_score() {
        let service = PasswordService::default();
        
        assert!(service.calculate_strength_score("12345678") < 50);
        assert!(service.calculate_strength_score("Test1234") >= 50);
        assert!(service.calculate_strength_score("Test1234!@#") >= 80);
    }
}

