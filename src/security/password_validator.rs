//! Password强度validate模块
//!
//! 提供企业级Password策略validate

use crate::core::errors::WalletError;

/// Password强度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrength {
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

/// Passwordvalidate配置
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_special: bool,
    pub min_strength: PasswordStrength,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: false,  // 可选，避免过于严格
            min_strength: PasswordStrength::Medium,
        }
    }
}

impl PasswordPolicy {
    /// 创建严格的Password策略（用于高安全场景）
    pub fn strict() -> Self {
        Self {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: true,
            min_strength: PasswordStrength::Strong,
        }
    }

    /// 创建宽松的Password策略（用于测试或低安全场景）
    pub fn lenient() -> Self {
        Self {
            min_length: 6,
            require_uppercase: false,
            require_lowercase: true,
            require_digit: false,
            require_special: false,
            min_strength: PasswordStrength::Weak,
        }
    }
}

/// validatePassword强度
///
/// # Arguments
/// * `password` - 待validate的Password
/// * `policy` - Password策略
///
/// # Returns
/// * `Ok(PasswordStrength)` - Password强度等级
/// * `Err(WalletError)` - validatefailed原因
pub fn validate_password(password: &str, policy: &PasswordPolicy) -> Result<PasswordStrength, WalletError> {
    // 1. checkPassword不为空
    if password.is_empty() {
        return Err(WalletError::SecurityError("Password不能为空".to_string()));
    }

    // 2. check最小长度
    if password.len() < policy.min_length {
        return Err(WalletError::SecurityError(
            format!("Password至少需要{}个字符", policy.min_length)
        ));
    }

    // 3. check必须包含大写字母
    if policy.require_uppercase && !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(WalletError::SecurityError("Password必须包含至少一个大写字母".to_string()));
    }

    // 4. check必须包含小写字母
    if policy.require_lowercase && !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(WalletError::SecurityError("Password必须包含至少一个小写字母".to_string()));
    }

    // 5. check必须包含数字
    if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(WalletError::SecurityError("Password必须包含至少一个数字".to_string()));
    }

    // 6. check必须包含特殊字符
    if policy.require_special {
        let has_special = password.chars().any(|c| {
            matches!(c, '!' | '@' | '#' | '$' | '%' | '^' | '&' | '*' | '(' | ')' | '-' | '_' | '=' | '+' | '[' | ']' | '{' | '}' | '|' | ';' | ':' | '\'' | '"' | '<' | '>' | ',' | '.' | '?' | '/' | '~' | '`')
        });
        if !has_special {
            return Err(WalletError::SecurityError("Password必须包含至少一个特殊字符".to_string()));
        }
    }

    // 7. check常见弱Password
    let weak_passwords = [
        "password", "123456", "12345678", "qwerty", "abc123",
        "password123", "admin", "letmein", "welcome", "monkey",
    ];
    let lower_password = password.to_lowercase();
    if weak_passwords.iter().any(|&weak| lower_password.contains(weak)) {
        return Err(WalletError::SecurityError("Password过于简单，请使用更复杂的Password".to_string()));
    }

    // 8. 计算Password强度
    let strength = calculate_password_strength(password);

    // 9. check是否达到最低强度要求
    if !meets_min_strength(strength, policy.min_strength) {
        return Err(WalletError::SecurityError(
            format!("Password强度不足，要求至少{:?}级别", policy.min_strength)
        ));
    }

    Ok(strength)
}

/// 计算Password强度
fn calculate_password_strength(password: &str) -> PasswordStrength {
    let mut score = 0;

    // 长度加分
    score += match password.len() {
        0..=7 => 0,
        8..=11 => 1,
        12..=15 => 2,
        _ => 3,
    };

    // 字符类型多样性加分
    if password.chars().any(|c| c.is_ascii_lowercase()) {
        score += 1;
    }
    if password.chars().any(|c| c.is_ascii_uppercase()) {
        score += 1;
    }
    if password.chars().any(|c| c.is_ascii_digit()) {
        score += 1;
    }
    if password.chars().any(|c| !c.is_alphanumeric()) {
        score += 2;  // 特殊字符加分更多
    }

    // 根据分数判断强度
    match score {
        0..=3 => PasswordStrength::Weak,
        4..=5 => PasswordStrength::Medium,
        6..=7 => PasswordStrength::Strong,
        _ => PasswordStrength::VeryStrong,
    }
}

/// check是否满足最低强度要求
fn meets_min_strength(actual: PasswordStrength, required: PasswordStrength) -> bool {
    let actual_score = match actual {
        PasswordStrength::Weak => 1,
        PasswordStrength::Medium => 2,
        PasswordStrength::Strong => 3,
        PasswordStrength::VeryStrong => 4,
    };
    let required_score = match required {
        PasswordStrength::Weak => 1,
        PasswordStrength::Medium => 2,
        PasswordStrength::Strong => 3,
        PasswordStrength::VeryStrong => 4,
    };
    actual_score >= required_score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_too_short() {
        let policy = PasswordPolicy::default();
        let result = validate_password("Short1", &policy);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::SecurityError(_)));
    }

    #[test]
    fn test_password_no_uppercase() {
        let policy = PasswordPolicy::default();
        let result = validate_password("lowercase123", &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_password_no_digit() {
        let policy = PasswordPolicy::default();
        let result = validate_password("NoDigitPass", &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_password_weak_common() {
        let policy = PasswordPolicy::default();
        let result = validate_password("Password123", &policy);
        assert!(result.is_err());  // 包含"password"
    }

    #[test]
    fn test_password_valid_medium() {
        let policy = PasswordPolicy::default();
        let result = validate_password("MySecure123", &policy);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PasswordStrength::Medium);
    }

    #[test]
    fn test_password_valid_strong() {
        let policy = PasswordPolicy::default();
        let result = validate_password("MyV3rySecur3P@ss", &policy);
        assert!(result.is_ok());
        let strength = result.unwrap();
        assert!(matches!(strength, PasswordStrength::Strong | PasswordStrength::VeryStrong));
    }

    #[test]
    fn test_password_empty() {
        let policy = PasswordPolicy::default();
        let result = validate_password("", &policy);
        assert!(result.is_err());
    }

    #[test]
    fn test_strict_policy() {
        let policy = PasswordPolicy::strict();
        let result = validate_password("Short1!", &policy);
        assert!(result.is_err());  // 太短（<12字符）
    }

    #[test]
    fn test_lenient_policy() {
        let policy = PasswordPolicy::lenient();
        let result = validate_password("simple", &policy);
        assert!(result.is_ok());
    }
}

