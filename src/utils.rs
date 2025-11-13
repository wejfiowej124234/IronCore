// src/utils.rs
use hex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtilsError {
    #[error("Invalid hex string: {0}")]
    InvalidHexString(String),
}

/// Convert a hex-encoded string to bytes.
pub fn hex_to_bytes(hex_string: &str) -> Result<Vec<u8>, UtilsError> {
    if hex_string.trim().is_empty() {
        return Err(UtilsError::InvalidHexString("Hex string cannot be empty".to_string()));
    }

    hex::decode(hex_string).map_err(|e| UtilsError::InvalidHexString(e.to_string()))
}

/// Convert bytes to a hex string.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_bytes() {
        let bytes = hex_to_bytes("48656c6c6f").unwrap();
        assert_eq!(bytes, b"Hello");
    }

    #[test]
    fn test_hex_to_bytes_invalid() {
        assert!(hex_to_bytes("invalid").is_err());
    }

    #[test]
    fn test_bytes_to_hex() {
        let hex = bytes_to_hex(b"Hello");
        assert_eq!(hex, "48656c6c6f");
    }
}
