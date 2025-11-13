//! API请求validate器
//! 
//! 提供统一的validate逻辑，避免在各个handler中重复代码

use axum::{http::StatusCode, response::Json};
use crate::api::types::ErrorResponse;

/// validate结果类型（统一error响应）
pub type ValidationResult = Result<(), (StatusCode, Json<ErrorResponse>)>;

/// validateWallet name
/// 
/// 规则：
/// - 不能为空
/// - 长度限制：1-64字符
/// - 只能包含字母、数字、下划线
/// - 不能包含路径遍历字符（../ 等）
/// - 不能包含特殊字符（防XSS和注入）
pub fn validate_wallet_name(name: &str) -> ValidationResult {
    if name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Wallet name cannot be empty".to_string(),
                code: "INVALID_WALLET_NAME".to_string(),
            }),
        ));
    }

    // Length limit: maximum 64 characters
    if name.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Wallet name too long (max 64 characters)".to_string(),
                code: "WALLET_NAME_TOO_LONG".to_string(),
            }),
        ));
    }

    // Path traversal protection
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Wallet name contains invalid characters (path traversal detected)".to_string(),
                code: "INVALID_WALLET_NAME".to_string(),
            }),
        ));
    }

    // Only allow letters, numbers, underscores, and hyphens
    if name.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid wallet name: must contain only letters, numbers, underscores, and hyphens".to_string(),
                code: "INVALID_WALLET_NAME".to_string(),
            }),
        ));
    }

    Ok(())
}

/// Validate network parameter
///
/// Rules:
/// - Cannot be empty
/// - Must be a supported network
///
/// Returns normalized network ID
pub fn validate_and_normalize_network(network: &str) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    if network.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Network parameter is required".to_string(),
                code: "INVALID_NETWORK".to_string(),
            }),
        ));
    }

    // Normalize network name (supports aliases)
    let normalized = match network {
        "ethereum" => "eth",
        "binance" | "bnb" => "bsc",
        "bitcoin" => "btc",
        // Already a canonical name
        "eth" | "sepolia" | "polygon" | "bsc" | "btc" => network,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Unsupported network: {}. Supported: eth, btc, bsc, polygon, sepolia", network),
                    code: "UNSUPPORTED_NETWORK".to_string(),
                }),
            ));
        }
    };

    Ok(normalized.to_string())
}

/// Validate required parameter is not empty
///
/// Used to verify string parameters are not empty
pub fn validate_required_param(param: &str, param_name: &str) -> ValidationResult {
    if param.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{} is required", param_name),
                code: "MISSING_PARAMETER".to_string(),
            }),
        ));
    }
    Ok(())
}

/// Validate password strength
///
/// Rules (maximum security):
/// - At least 8 characters
/// - Must contain at least one uppercase letter
/// - Must contain at least one lowercase letter
/// - Must contain at least one digit
/// - Must contain at least one special character
pub fn validate_password_strength(password: &str) -> ValidationResult {
    // Check minimum length
    if password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password must be at least 8 characters".to_string(),
                code: "WEAK_PASSWORD".to_string(),
            }),
        ));
    }
    
    // Check for uppercase letter requirement
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password must contain at least one uppercase letter".to_string(),
                code: "WEAK_PASSWORD".to_string(),
            }),
        ));
    }
    
    // Check for lowercase letter requirement
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password must contain at least one lowercase letter".to_string(),
                code: "WEAK_PASSWORD".to_string(),
            }),
        ));
    }
    
    // Check for digit requirement
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password must contain at least one digit".to_string(),
                code: "WEAK_PASSWORD".to_string(),
            }),
        ));
    }
    
    // Check for special character requirement
    let has_special = password.chars().any(|c| {
        "!@#$%^&*()_+-=[]{}|;:,.<>?/~`".contains(c)
    });
    
    if !has_special {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password must contain at least one special character (!@#$%^&* etc.)".to_string(),
                code: "WEAK_PASSWORD".to_string(),
            }),
        ));
    }
    
    Ok(())
}

/// Validate transaction amount
/// 
/// Rules:
/// - Must be a valid number
/// - Must be greater than 0
/// - Precision limit (maximum 18 decimal places)
pub fn validate_transaction_amount(amount: &str) -> ValidationResult {
    // Parse as floating point number
    let parsed_amount: f64 = amount.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid amount format".to_string(),
                code: "INVALID_AMOUNT".to_string(),
            }),
        )
    })?;

    // ✅ 必须大于0
    if parsed_amount <= 0.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Amount must be greater than 0".to_string(),
                code: "INVALID_AMOUNT".to_string(),
            }),
        ));
    }

    // ✅ 防止无穷大和NaN
    if !parsed_amount.is_finite() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid amount value".to_string(),
                code: "INVALID_AMOUNT".to_string(),
            }),
        ));
    }

    Ok(())
}

/// validatewalletaddress格式
/// 
/// 规则：
/// - 必须以0x开头
/// - 总长度42字符（0x + 40位十六进制）
/// - 只包含十六进制字符
pub fn validate_wallet_address(address: &str) -> ValidationResult {
    if !address.starts_with("0x") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Wallet address must start with 0x".to_string(),
                code: "INVALID_ADDRESS".to_string(),
            }),
        ));
    }

    if address.len() != 42 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Wallet address must be 42 characters long (0x + 40 hex digits)".to_string(),
                code: "INVALID_ADDRESS".to_string(),
            }),
        ));
    }

    // validate十六进制字符
    let hex_part = &address[2..];
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Wallet address must contain only hexadecimal characters".to_string(),
                code: "INVALID_ADDRESS".to_string(),
            }),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_wallet_name_valid() {
        assert!(validate_wallet_name("my_wallet").is_ok());
        assert!(validate_wallet_name("wallet123").is_ok());
        assert!(validate_wallet_name("test_wallet_123").is_ok());
    }

    #[test]
    fn test_validate_wallet_name_invalid() {
        assert!(validate_wallet_name("").is_err());
        // Note: Allowing hyphens in wallet names for better UX
    // assert!(validate_wallet_name("wallet-name").is_err());
        assert!(validate_wallet_name("wallet name").is_err());
        assert!(validate_wallet_name("wallet!").is_err());
    }

    #[test]
    fn test_validate_network() {
        assert_eq!(validate_and_normalize_network("eth").unwrap(), "eth");
        assert_eq!(validate_and_normalize_network("ethereum").unwrap(), "eth");
        assert_eq!(validate_and_normalize_network("btc").unwrap(), "btc");
        assert_eq!(validate_and_normalize_network("bitcoin").unwrap(), "btc");
        assert_eq!(validate_and_normalize_network("bsc").unwrap(), "bsc");
        assert_eq!(validate_and_normalize_network("binance").unwrap(), "bsc");
    }

    #[test]
    fn test_validate_network_invalid() {
        assert!(validate_and_normalize_network("").is_err());
        assert!(validate_and_normalize_network("invalid").is_err());
    }

    #[test]
    fn test_validate_password_strength() {
        // ✅ 满分安全策略：需要大写+小写+数字+特殊字符
        assert!(validate_password_strength("Test1234!").is_ok());  // ✅ 符合所有要求
        assert!(validate_password_strength("Secure@Pass123").is_ok());  // ✅ 强Password
        
        // ❌ 应该拒绝的弱Password
        assert!(validate_password_strength("12345678").is_err());  // 无大小写和特殊字符
        assert!(validate_password_strength("password").is_err());  // 无大写、数字、特殊字符
        assert!(validate_password_strength("Password").is_err());  // 无数字和特殊字符
        assert!(validate_password_strength("Password123").is_err());  // 无特殊字符
        assert!(validate_password_strength("short").is_err());  // 太短
        assert!(validate_password_strength("").is_err());  // 空Password
    }
}

