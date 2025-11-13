use std::fmt;

/// Custom error type for wallet operations.
#[derive(Debug)]
pub enum WalletError {
    /// Configuration-related errors.
    ConfigError(String),
    /// Storage-related errors.
    StorageError(String),
    /// Blockchain interaction errors.
    BlockchainError(String),
    /// Encryption/decryption errors.
    CryptoError(String),
    /// Security-related errors.
    SecurityError(String),
    /// Bridge operation errors.
    BridgeError(String),
    /// Validation errors.
    ValidationError(String),
    /// Network errors.
    NetworkError(String),
    /// Mnemonic generation/parsing errors.
    MnemonicError(String),
    /// Key derivation errors.
    KeyDerivationError(String),
    /// Address derivation errors.
    AddressError(String),
    /// Serialization/deserialization errors.
    SerializationError(String),
    /// Resource not found errors.
    NotFoundError(String),
    /// Not implemented functionality.
    NotImplemented(String),
    /// Internal errors.
    InternalError(String),
    /// Invalid address errors.
    InvalidAddress(String),
    /// Invalid private key errors.
    InvalidPrivateKey(String),
    /// Invalid amount errors (Bitcoin).
    InvalidAmount(String),
    /// Insufficient funds errors.
    InsufficientFunds(String),
    /// Signing failed errors.
    SigningFailed(String),
    /// Key generation failed errors.
    KeyGenerationFailed(String),
    /// Address generation failed errors.
    AddressGenerationFailed(String),
    /// Transaction failed errors.
    TransactionFailed(String),
    /// Timeout errors.
    TimeoutError(String),
    /// Encryption errors (specific).
    EncryptionError(String),
    /// Decryption errors (specific).
    DecryptionError(String),
    /// Async operation errors.
    AsyncError(String),
    /// Invalid input errors.
    InvalidInput(String),
    /// Memory errors.
    MemoryError(String),
    /// IO errors (specific).
    IoError(String),
    /// Deserialization errors.
    DeserializationError(String),
    /// Generic errors.
    GenericError(String),
    /// Generic errors (legacy).
    Other(String),
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            WalletError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            WalletError::BlockchainError(msg) => write!(f, "Blockchain error: {}", msg),
            WalletError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
            WalletError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            WalletError::BridgeError(msg) => write!(f, "Bridge error: {}", msg),
            WalletError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            WalletError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            WalletError::MnemonicError(msg) => write!(f, "Mnemonic error: {}", msg),
            WalletError::KeyDerivationError(msg) => write!(f, "Key derivation error: {}", msg),
            WalletError::AddressError(msg) => write!(f, "Address error: {}", msg),
            WalletError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            WalletError::NotFoundError(msg) => write!(f, "Not found: {}", msg),
            WalletError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            WalletError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            WalletError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            WalletError::InvalidPrivateKey(msg) => write!(f, "Invalid private key: {}", msg),
            WalletError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            WalletError::InsufficientFunds(msg) => write!(f, "Insufficient funds: {}", msg),
            WalletError::SigningFailed(msg) => write!(f, "Signing failed: {}", msg),
            WalletError::KeyGenerationFailed(msg) => write!(f, "Key generation failed: {}", msg),
            WalletError::AddressGenerationFailed(msg) => write!(f, "Address generation failed: {}", msg),
            WalletError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            WalletError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            WalletError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            WalletError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            WalletError::AsyncError(msg) => write!(f, "Async error: {}", msg),
            WalletError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            WalletError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            WalletError::IoError(msg) => write!(f, "IO error: {}", msg),
            WalletError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            WalletError::GenericError(msg) => write!(f, "Error: {}", msg),
            WalletError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for WalletError {}

impl WalletError {
    /// 创建一个通用error
    pub fn new(message: impl Into<String>) -> Self {
        Self::GenericError(message.into())
    }

    /// 判断是否为关键error
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            WalletError::SecurityError(_)
                | WalletError::CryptoError(_)
        )
    }

    /// 判断是否为可重试error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            WalletError::NetworkError(_)
                | WalletError::TimeoutError(_)
        )
    }
}

impl From<anyhow::Error> for WalletError {
    fn from(err: anyhow::Error) -> Self {
        WalletError::Other(err.to_string())
    }
}

impl From<std::io::Error> for WalletError {
    fn from(err: std::io::Error) -> Self {
        WalletError::StorageError(err.to_string())
    }
}

impl From<serde_json::Error> for WalletError {
    fn from(err: serde_json::Error) -> Self {
        WalletError::ValidationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_config_error() {
        let err = WalletError::ConfigError("Invalid config".to_string());
        assert_eq!(format!("{}", err), "Configuration error: Invalid config");
    }

    #[test]
    fn test_display_storage_error() {
        let err = WalletError::StorageError("DB failure".to_string());
        assert_eq!(format!("{}", err), "Storage error: DB failure");
    }

    #[test]
    fn test_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("Test error");
        let wallet_err: WalletError = anyhow_err.into();
        match wallet_err {
            WalletError::Other(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Other variant"),
        }
    }
}
