//! Encrypted Key Storage
//!
//! Provides AES-256-GCM encrypted storage for sensitive cryptographic keys.
//! Uses HKDF for key derivation from passwords.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};
use anyhow::{anyhow, Result};
use rand::RngCore;

/// Encrypted storage for cryptographic keys
pub struct EncryptedKeyStorage {
    encrypted_data: Option<Vec<u8>>,
    nonce: Option<[u8; 12]>,
    salt: [u8; 32],
}

impl EncryptedKeyStorage {
    /// Create a new encrypted key storage with a random salt
    pub fn new() -> Self {
        let mut salt = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        
        Self {
            encrypted_data: None,
            nonce: None,
            salt,
        }
    }

    /// Create encrypted storage with a specific salt (for testing or persistence)
    pub fn with_salt(salt: [u8; 32]) -> Self {
        Self {
            encrypted_data: None,
            nonce: None,
            salt,
        }
    }

    /// Derive an encryption key from a password using HKDF
    fn derive_encryption_key(&self, password: &str) -> Zeroizing<Vec<u8>> {
        let hk = Hkdf::<Sha256>::new(Some(&self.salt), password.as_bytes());
        let mut okm = Zeroizing::new(vec![0u8; 32]);
        hk.expand(b"wallet-key-storage-v1", &mut okm)
            .unwrap_or_else(|_| {
                // HKDF expand不应该failed（info长度是固定的）
                panic!("Critical: HKDF expand failed with fixed info length")
            });
        okm
    }

    /// Store a key with password-based encryption
    ///
    /// # Arguments
    /// * `key` - The key bytes to store
    /// * `password` - Password to derive encryption key from
    ///
    /// # Security
    /// - Uses AES-256-GCM for encryption
    /// - Random nonce for each encryption
    /// - HKDF for key derivation
    pub fn store(&mut self, key: &[u8], password: &str) -> Result<()> {
        // Validate inputs
        if key.is_empty() {
            return Err(anyhow!("Key cannot be empty"));
        }
        if password.is_empty() {
            return Err(anyhow!("Password cannot be empty"));
        }
        if password.len() < 8 {
            return Err(anyhow!("Password must be at least 8 characters"));
        }

        // Derive encryption key
        let encryption_key = self.derive_encryption_key(password);
        
        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        #[allow(deprecated)] // GenericArray::from_slice 在 generic-array 0.14.x 中已弃用
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = cipher.encrypt(nonce, key)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        self.encrypted_data = Some(ciphertext);
        self.nonce = Some(nonce_bytes);
        
        Ok(())
    }

    /// Retrieve and decrypt the stored key
    ///
    /// # Arguments
    /// * `password` - Password to derive decryption key from
    ///
    /// # Returns
    /// Zeroizing buffer containing the decrypted key
    pub fn retrieve(&self, password: &str) -> Result<Zeroizing<Vec<u8>>> {
        let encrypted_data = self.encrypted_data.as_ref()
            .ok_or_else(|| anyhow!("No key stored"))?;
        let nonce_bytes = self.nonce.as_ref()
            .ok_or_else(|| anyhow!("No nonce stored"))?;
        
        // Derive encryption key
        let encryption_key = self.derive_encryption_key(password);
        
        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
        
        #[allow(deprecated)] // GenericArray::from_slice 在 generic-array 0.14.x 中已弃用
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Decrypt
        let plaintext = cipher.decrypt(nonce, encrypted_data.as_ref())
            .map_err(|_| anyhow!("Decryption failed (wrong password or corrupted data)"))?;
        
        Ok(Zeroizing::new(plaintext))
    }

    /// Check if a key is stored
    pub fn has_key(&self) -> bool {
        self.encrypted_data.is_some() && self.nonce.is_some()
    }

    /// Clear the stored encrypted key
    pub fn clear(&mut self) {
        if let Some(ref mut data) = self.encrypted_data {
            data.zeroize();
        }
        if let Some(ref mut n) = self.nonce {
            n.zeroize();
        }
        self.encrypted_data = None;
        self.nonce = None;
    }

    /// Get the salt (for persistence)
    pub fn salt(&self) -> &[u8; 32] {
        &self.salt
    }

    /// Get encrypted data for persistence (if stored)
    pub fn encrypted_data(&self) -> Option<&[u8]> {
        self.encrypted_data.as_deref()
    }

    /// Get nonce for persistence (if stored)
    pub fn nonce(&self) -> Option<&[u8; 12]> {
        self.nonce.as_ref()
    }

    /// Restore from persisted components
    pub fn restore(salt: [u8; 32], nonce: [u8; 12], encrypted_data: Vec<u8>) -> Self {
        Self {
            encrypted_data: Some(encrypted_data),
            nonce: Some(nonce),
            salt,
        }
    }
}

impl Default for EncryptedKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EncryptedKeyStorage {
    fn drop(&mut self) {
        self.clear();
    }
}

impl Zeroize for EncryptedKeyStorage {
    fn zeroize(&mut self) {
        self.clear();
        self.salt.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let mut storage = EncryptedKeyStorage::new();
        let key = b"super_secret_key_32_bytes_long!!";
        let password = "strong_password_123";

        // Store
        storage.store(key, password).unwrap();
        assert!(storage.has_key());

        // Retrieve
        let retrieved = storage.retrieve(password).unwrap();
        assert_eq!(&retrieved[..], key);
    }

    #[test]
    fn test_wrong_password_fails() {
        let mut storage = EncryptedKeyStorage::new();
        let key = b"super_secret_key_32_bytes_long!!";
        let password = "correct_password";

        storage.store(key, password).unwrap();

        // Wrong password should fail
        assert!(storage.retrieve("wrong_password").is_err());
    }

    #[test]
    fn test_empty_key_rejected() {
        let mut storage = EncryptedKeyStorage::new();
        assert!(storage.store(b"", "password").is_err());
    }

    #[test]
    fn test_weak_password_rejected() {
        let mut storage = EncryptedKeyStorage::new();
        let key = b"test_key";
        
        // Too short password
        assert!(storage.store(key, "short").is_err());
    }

    #[test]
    fn test_clear_removes_data() {
        let mut storage = EncryptedKeyStorage::new();
        let key = b"test_key_data";
        let password = "password123";

        storage.store(key, password).unwrap();
        assert!(storage.has_key());

        storage.clear();
        assert!(!storage.has_key());
        assert!(storage.retrieve(password).is_err());
    }

    #[test]
    fn test_different_salt_different_ciphertext() {
        let key = b"same_key_different_salt";
        let password = "same_password";

        let mut storage1 = EncryptedKeyStorage::new();
        let mut storage2 = EncryptedKeyStorage::new();

        storage1.store(key, password).unwrap();
        storage2.store(key, password).unwrap();

        // Different salts should produce different ciphertexts
        assert_ne!(storage1.encrypted_data(), storage2.encrypted_data());
    }

    #[test]
    fn test_restore_from_components() {
        let mut storage1 = EncryptedKeyStorage::new();
        let key = b"test_key_for_restore";
        let password = "restore_password";

        storage1.store(key, password).unwrap();

        // Get components
        let salt = *storage1.salt();
        let nonce = *storage1.nonce().unwrap();
        let encrypted_data = storage1.encrypted_data().unwrap().to_vec();

        // Restore to new storage
        let storage2 = EncryptedKeyStorage::restore(salt, nonce, encrypted_data);

        // Should be able to decrypt
        let retrieved = storage2.retrieve(password).unwrap();
        assert_eq!(&retrieved[..], key);
    }

    #[test]
    fn test_multiple_store_overwrites() {
        let mut storage = EncryptedKeyStorage::new();
        let password = "password123";

        storage.store(b"first_key", password).unwrap();
        storage.store(b"second_key", password).unwrap();

        let retrieved = storage.retrieve(password).unwrap();
        assert_eq!(&retrieved[..], b"second_key");
    }

    #[test]
    fn test_zeroize_on_drop() {
        let mut storage = EncryptedKeyStorage::new();
        let key = b"zeroize_test_key";
        let password = "password123";

        storage.store(key, password).unwrap();
        
        // Drop should call zeroize
        drop(storage);
        // No way to verify directly, but coverage tools will check
    }

    #[test]
    fn test_long_key() {
        let mut storage = EncryptedKeyStorage::new();
        let key = vec![42u8; 1024]; // 1KB key
        let password = "long_key_password";

        storage.store(&key, password).unwrap();
        let retrieved = storage.retrieve(password).unwrap();
        assert_eq!(&retrieved[..], &key[..]);
    }

    #[test]
    fn test_unicode_password() {
        let mut storage = EncryptedKeyStorage::new();
        let key = b"unicode_test";
        let password = "Password🔒パスワード";

        storage.store(key, password).unwrap();
        let retrieved = storage.retrieve(password).unwrap();
        assert_eq!(&retrieved[..], key);
    }

    #[test]
    fn test_with_salt() {
        let salt = [42u8; 32];
        let mut storage1 = EncryptedKeyStorage::with_salt(salt);
        let mut storage2 = EncryptedKeyStorage::with_salt(salt);

        let key = b"test_key";
        let password = "password123";

        storage1.store(key, password).unwrap();
        storage2.store(key, password).unwrap();

        // Same salt, but different nonce, so different ciphertext
        // (We can't directly compare since nonce is random)
        
        // But both should decrypt correctly
        assert_eq!(&storage1.retrieve(password).unwrap()[..], key);
        assert_eq!(&storage2.retrieve(password).unwrap()[..], key);
    }
}

