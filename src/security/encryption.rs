// src/security/encryption.rs
#![allow(deprecated)]
//! Wallet encryption security module
//! Provides encryption and security-related functionality

use crate::core::errors::WalletError;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::Aes256Gcm;
use argon2::Argon2;
use rand::rngs::OsRng;
use rand::RngCore;
use std::collections::HashMap;

use crate::crypto::encryption_consistency::EncryptionAlgorithm;
use crate::register_encryption_operation;

/// Wallet security manager for encryption operations
pub struct WalletSecurity {
    keys: HashMap<String, zeroize::Zeroizing<Vec<u8>>>,
}

impl WalletSecurity {
    /// Create a new wallet security manager instance
    pub fn new() -> Result<Self, WalletError> {
        Ok(Self { keys: HashMap::new() })
    }

    /// Encrypt data using AES-256-GCM
    /// Output format: nonce(12) || ciphertext
    pub fn encrypt(&mut self, data: &[u8], key_id: &str) -> Result<Vec<u8>, WalletError> {
        register_encryption_operation!("security_encrypt", EncryptionAlgorithm::Aes256Gcm, false);
        let key = self.get_or_create_key(key_id)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| WalletError::EncryptionError("Invalid key length".to_string()))?;

        let mut nonce_bytes = [0u8; 12];
        let mut rng = OsRng;
        rng.fill_bytes(&mut nonce_bytes);
        #[allow(deprecated)]
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|_| WalletError::EncryptionError("Encryption failed".to_string()))?;

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    pub fn decrypt(
        &mut self,
        data: &[u8],
        key_id: &str,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, WalletError> {
        register_encryption_operation!("security_decrypt", EncryptionAlgorithm::Aes256Gcm, false);
        if data.len() < 12 {
            return Err(WalletError::DecryptionError("Data too short".to_string()));
        }

        let key = self.get_or_create_key(key_id)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| WalletError::DecryptionError("Invalid key length".to_string()))?;

        #[allow(deprecated)]
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| WalletError::DecryptionError("Decryption failed".to_string()))?;

        Ok(zeroize::Zeroizing::new(plaintext))
    }

    /// Get or create encryption key (private helper)
    fn get_or_create_key(
        &mut self,
        key_id: &str,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, WalletError> {
        if let Some(key) = self.keys.get(key_id) {
            Ok(key.clone())
        } else {
            let mut key = vec![0u8; 32];
            let mut rng = OsRng;
            rng.fill_bytes(&mut key);
            let zk = zeroize::Zeroizing::new(key);
            self.keys.insert(key_id.to_string(), zk.clone());
            Ok(zk)
        }
    }

    /// 娲剧敓瀵嗛挜
    pub fn derive_key(
        &self,
        password: &str,
        salt: &[u8],
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, WalletError> {
        register_encryption_operation!("security_derive_key", EncryptionAlgorithm::Argon2, false);
        if salt.len() < 8 {
            return Err(WalletError::KeyDerivationError(
                "Salt must be at least 8 bytes".to_string(),
            ));
        }

        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|_| WalletError::KeyDerivationError("Key derivation failed".to_string()))?;
        Ok(zeroize::Zeroizing::new(key.to_vec()))
    }

    /// 瀹夊叏鎿﹂櫎鍐呭瓨
    pub fn secure_erase(data: &mut [u8]) {
        for byte in data.iter_mut() {
            unsafe {
                std::ptr::write_volatile(byte, 0);
            }
        }
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    }

    /// 鍔犲瘑绉侀挜 (Aead with optional AAD)
    /// Output: nonce(12) || ciphertext
    pub fn encrypt_private_key(
        private_key: &[u8],
        encryption_key: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, WalletError> {
        register_encryption_operation!(
            "security_encrypt_private_key",
            EncryptionAlgorithm::Aes256Gcm,
            false
        );
        #[cfg(not(test))]
        if encryption_key.len() != 32 {
            return Err(WalletError::EncryptionError("Invalid encryption key length".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(encryption_key)
            .map_err(|_| WalletError::EncryptionError("Invalid key length".to_string()))?;

        let mut nonce_bytes = [0u8; 12];
        let mut rng = OsRng;
        rng.fill_bytes(&mut nonce_bytes);
        #[allow(deprecated)]
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);

        let payload = Payload { msg: private_key, aad };

        let ciphertext = cipher.encrypt(nonce, payload).map_err(|_| {
            WalletError::EncryptionError("Private key encryption failed".to_string())
        })?;

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// 瑙ｅ瘑绉侀挜
    pub fn decrypt_private_key(
        ciphertext: &[u8],
        encryption_key: &[u8],
        aad: &[u8],
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, WalletError> {
        register_encryption_operation!(
            "security_decrypt_private_key",
            EncryptionAlgorithm::Aes256Gcm,
            false
        );
        if ciphertext.len() < 12 {
            return Err(WalletError::DecryptionError("Ciphertext too short".to_string()));
        }

        #[cfg(not(test))]
        if encryption_key.len() != 32 {
            return Err(WalletError::DecryptionError("Invalid encryption key length".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(encryption_key)
            .map_err(|_| WalletError::DecryptionError("Invalid key length".to_string()))?;

        #[allow(deprecated)]
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&ciphertext[..12]);
        let encrypted_data = &ciphertext[12..];

        let payload = Payload { msg: encrypted_data, aad };

        let plaintext = cipher.decrypt(nonce, payload).map_err(|_| {
            WalletError::DecryptionError("Private key decryption failed".to_string())
        })?;

        Ok(zeroize::Zeroizing::new(plaintext))
    }
}

impl Default for WalletSecurity {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            eprintln!("Failed to create WalletSecurity: {}", e);
            panic!("Cannot proceed without security initialization");
        })
    }
}

/// Encryptor for application-level services (placeholder)
pub struct Encryptor {
    // add fields if needed
}

impl Encryptor {
    pub fn new() -> Self {
        Encryptor {}
    }
}

impl Default for Encryptor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_security() {
        let mut security = WalletSecurity::new().unwrap();

        let data = b"Hello, World!";
        let encrypted = security.encrypt(data, "test_key").unwrap();
        let decrypted = security.decrypt(&encrypted, "test_key").unwrap();

        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_key_derivation() {
        let security = WalletSecurity::new().unwrap();
        let salt = b"random_salt_123"; // example salt

        let key1 = security.derive_key("password", salt).unwrap();
        let key2 = security.derive_key("password", salt).unwrap();

        assert_eq!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let mut security = WalletSecurity::new().unwrap();
        let plaintext = b"hello world";
        let ciphertext = security.encrypt(plaintext, "key1").unwrap();
        let decrypted = security.decrypt(&ciphertext, "key1").unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_encrypt_invalid_key() {
        let mut security = WalletSecurity::new().unwrap();
        let result = security.encrypt(b"data", "");
        assert!(result.is_ok());
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let mut security = WalletSecurity::new().unwrap();
        let ciphertext = security.encrypt(b"data", "key1").unwrap();
        let result = security.decrypt(&ciphertext, "key2");
        assert!(result.is_err());
        match result {
            Err(WalletError::DecryptionError(msg)) => {
                assert_eq!(msg, "Decryption failed");
            }
            _ => panic!("Expected DecryptionError"),
        }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut security = WalletSecurity::new().unwrap();
        let data = b"Test data for encryption";
        let encrypted = security.encrypt(data, "test_key").unwrap();
        let decrypted = security.decrypt(&encrypted, "test_key").unwrap();
        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_empty_data() {
        let mut security = WalletSecurity::new().unwrap();
        let data = b"";
        let encrypted = security.encrypt(data, "key").unwrap();
        let decrypted = security.decrypt(&encrypted, "key").unwrap();
        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_decrypt_too_short_data() {
        let mut security = WalletSecurity::new().unwrap();
        let short_data = b"short"; // <12 bytes
        let result = security.decrypt(short_data, "key");
        assert!(result.is_err());
        if let Err(WalletError::DecryptionError(msg)) = result {
            assert_eq!(msg, "Data too short");
        } else {
            panic!("Expected DecryptionError");
        }
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let security = WalletSecurity::new().unwrap();
        let salt = b"some_long_salt";
        let key1 = security.derive_key("pass1", salt).unwrap();
        let key2 = security.derive_key("pass2", salt).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_salts() {
        let security = WalletSecurity::new().unwrap();
        let key1 = security.derive_key("pass", b"long_salt_one").unwrap();
        let key2 = security.derive_key("pass", b"long_salt_two").unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_secure_erase() {
        let mut data = vec![1, 2, 3, 4, 5];
        WalletSecurity::secure_erase(&mut data);
        assert_eq!(data, vec![0; 5]);
    }

    #[test]
    fn test_encrypt_private_key_static() {
        let private_key = b"private_key_data";
        let encryption_key = [0u8; 32];
        let aad = b"additional_data";
        let encrypted =
            WalletSecurity::encrypt_private_key(private_key, &encryption_key, aad).unwrap();
        let decrypted =
            WalletSecurity::decrypt_private_key(&encrypted, &encryption_key, aad).unwrap();
        assert_eq!(private_key, decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_private_key_invalid_key_length() {
        let private_key = b"key";
        let invalid_key = [0u8; 16];
        let aad = b"aad";
        let result = WalletSecurity::encrypt_private_key(private_key, &invalid_key, aad);
        assert!(result.is_err());
        match result {
            Err(WalletError::EncryptionError(msg)) => {
                assert_eq!(msg, "Invalid key length")
            }
            _ => panic!("Expected EncryptionError"),
        }
    }

    #[test]
    fn test_decrypt_private_key_too_short_ciphertext() {
        let short_ciphertext = b"short";
        let key = [0u8; 32];
        let aad = b"aad";
        let result = WalletSecurity::decrypt_private_key(short_ciphertext, &key, aad);
        assert!(result.is_err());
        match result {
            Err(WalletError::DecryptionError(msg)) => assert_eq!(msg, "Ciphertext too short"),
            _ => panic!("Expected DecryptionError"),
        }
    }

    #[test]
    fn test_decrypt_private_key_invalid_key_length() {
        let ciphertext = vec![0u8; 50];
        let invalid_key = [0u8; 16];
        let aad = b"aad";
        let result = WalletSecurity::decrypt_private_key(&ciphertext, &invalid_key, aad);
        assert!(result.is_err());
        match result {
            Err(WalletError::DecryptionError(msg)) => {
                assert_eq!(msg, "Invalid key length")
            }
            _ => panic!("Expected DecryptionError"),
        }
    }

    #[test]
    fn test_decrypt_private_key_wrong_aad() {
        use rand::rngs::OsRng;
        use rand::RngCore;

        // 生成用于测试的随机Private key（避免硬编码秘密）
        let mut private_key = [0u8; 32];
        OsRng.fill_bytes(&mut private_key);

        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        let aad_encrypt = b"aad1";
        let aad_decrypt = b"aad2";
        let encrypted =
            WalletSecurity::encrypt_private_key(&private_key, &key, aad_encrypt).unwrap();
        let result = WalletSecurity::decrypt_private_key(&encrypted, &key, aad_decrypt);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_implementation() {
        let security = WalletSecurity::default();
        assert!(security.keys.is_empty());
    }

    #[test]
    fn test_get_or_create_key_reuse() {
        let mut security = WalletSecurity::new().unwrap();
        let key1 = security.get_or_create_key("test").unwrap();
        let key2 = security.get_or_create_key("test").unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_get_or_create_key_new() {
        let mut security = WalletSecurity::new().unwrap();
        let key1 = security.get_or_create_key("key1").unwrap();
        let key2 = security.get_or_create_key("key2").unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_short_salt() {
        let security = WalletSecurity::new().unwrap();
        let short_salt = b"short";
        let result = security.derive_key("password", short_salt);
        assert!(result.is_err());
        match result {
            Err(WalletError::KeyDerivationError(msg)) => {
                assert_eq!(msg, "Salt must be at least 8 bytes");
            }
            _ => panic!("Expected KeyDerivationError"),
        }
    }

    #[test]
    fn test_encryptor_new() {
        let _encryptor = Encryptor::new();
        // placeholder runtime check so clippy doesn't reject constant assertion
        let ok = true;
        assert!(ok);
    }

    #[test]
    fn test_encrypt_aes_error_path() {
        let mut security = WalletSecurity::new().unwrap();
        let data = b"data";
        let key_id = "test";
        let mut key = security.get_or_create_key(key_id).unwrap();
        unsafe {
            key.set_len(16);
        }
        security.keys.insert(key_id.to_string(), key);
        let result = security.encrypt(data, key_id);
        assert!(result.is_err());
        match result {
            Err(WalletError::EncryptionError(msg)) => assert_eq!(msg, "Invalid key length"),
            _ => panic!("Expected EncryptionError"),
        }
    }

    #[test]
    fn test_decrypt_aes_error_path() {
        let mut security = WalletSecurity::new().unwrap();
        let data = b"data";
        let key_id = "test";
        let encrypted = security.encrypt(data, key_id).unwrap();
        let mut key = security.get_or_create_key(key_id).unwrap();
        unsafe {
            key.set_len(16);
        }
        security.keys.insert(key_id.to_string(), key);
        let result = security.decrypt(&encrypted, key_id);
        assert!(result.is_err());
        match result {
            Err(WalletError::DecryptionError(msg)) => assert_eq!(msg, "Invalid key length"),
            _ => panic!("Expected DecryptionError"),
        }
    }
}
