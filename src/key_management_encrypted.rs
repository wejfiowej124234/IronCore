//! Encrypted Key Management
//!
//! Provides secure, encrypted key storage using AES-256-GCM

use crate::security::EncryptedKeyStorage;
use lazy_static::lazy_static;
use std::sync::Mutex;
use zeroize::Zeroizing;

pub type SecretVec = Zeroizing<Vec<u8>>;

lazy_static! {
    /// Global encrypted key storage
    static ref ENCRYPTED_KEY_STORAGE: Mutex<EncryptedKeyStorage> = 
        Mutex::new(EncryptedKeyStorage::new());
}

#[derive(Debug, thiserror::Error)]
pub enum EncryptedKeyError {
    #[error("Key storage failed: {0}")]
    StorageFailed(String),
    #[error("Key not found or wrong password")]
    KeyNotFound,
    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

/// Store a key with password-based encryption
///
/// # Arguments
/// * `key` - The key bytes to store
/// * `password` - Password to encrypt the key (min 8 characters)
///
/// # Security
/// - Uses AES-256-GCM encryption
/// - HKDF key derivation from password
/// - Random salt and nonce
pub fn store_key_encrypted(key: &[u8], password: &str) -> Result<(), EncryptedKeyError> {
    let mut storage = ENCRYPTED_KEY_STORAGE
        .lock()
        .map_err(|e| EncryptedKeyError::StorageFailed(e.to_string()))?;
    
    storage.store(key, password)
        .map_err(|e| EncryptedKeyError::StorageFailed(e.to_string()))
}

/// Retrieve an encrypted key
///
/// # Arguments
/// * `password` - Password to decrypt the key
///
/// # Returns
/// Zeroizing buffer containing the decrypted key
pub fn retrieve_key_encrypted(password: &str) -> Result<SecretVec, EncryptedKeyError> {
    let storage = ENCRYPTED_KEY_STORAGE
        .lock()
        .map_err(|e| EncryptedKeyError::StorageFailed(e.to_string()))?;
    
    storage.retrieve(password)
        .map_err(|_| EncryptedKeyError::KeyNotFound)
}

/// Check if a key is stored
pub fn has_encrypted_key() -> bool {
    ENCRYPTED_KEY_STORAGE
        .lock()
        .map(|storage| storage.has_key())
        .unwrap_or(false)
}

/// Clear the encrypted key storage
pub fn clear_encrypted_storage() -> Result<(), EncryptedKeyError> {
    let mut storage = ENCRYPTED_KEY_STORAGE
        .lock()
        .map_err(|e| EncryptedKeyError::StorageFailed(e.to_string()))?;
    
    storage.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve_encrypted() {
        let key = b"test_secret_key_32_bytes_long!!!";
        let password = "test_password_123";

        // Store
        store_key_encrypted(key, password).unwrap();
        assert!(has_encrypted_key());

        // Retrieve
        let retrieved = retrieve_key_encrypted(password).unwrap();
        assert_eq!(&retrieved[..], key);

        // Cleanup
        clear_encrypted_storage().unwrap();
    }

    #[test]
    fn test_wrong_password() {
        let key = b"another_test_key";
        let password = "correct_password";

        store_key_encrypted(key, password).unwrap();

        // Wrong password should fail
        assert!(retrieve_key_encrypted("wrong_password").is_err());

        // Cleanup
        clear_encrypted_storage().unwrap();
    }

    #[test]
    fn test_clear_storage() {
        let key = b"temp_key";
        let password = "temp_pass";

        store_key_encrypted(key, password).unwrap();
        assert!(has_encrypted_key());

        clear_encrypted_storage().unwrap();
        assert!(!has_encrypted_key());
    }
}

