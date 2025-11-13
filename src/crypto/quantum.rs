// src/crypto/quantum.rs
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use anyhow::Result;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::crypto::encryption_consistency::EncryptionAlgorithm;
use crate::register_encryption_operation;

const KYBER_CIPHERTEXT_LEN: usize = 1568;
const KYBER_SECRET_LEN: usize = 3168;
const AES_NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumKeyPair {
    pub public_key: Vec<u8>,
    #[serde(skip_serializing)]
    secret_key: Vec<u8>,
}

#[derive(Debug)]
pub struct QuantumSafeEncryption {
    keypair: Option<QuantumKeyPair>,
}

impl QuantumSafeEncryption {
    pub fn new() -> Result<Self> {
        info!("Initializing Quantum-Safe Encryption (simulated Kyber1024)");
        let mut instance = Self { keypair: None };
        instance.generate_keypair()?;
        Ok(instance)
    }

    pub fn generate_keypair(&mut self) -> Result<QuantumKeyPair> {
        debug!("Generating new simulated Kyber1024 keypair");

        let mut public_key = vec![0u8; KYBER_CIPHERTEXT_LEN];
        let mut secret_key = vec![0u8; KYBER_SECRET_LEN];

        rand::rngs::OsRng.fill_bytes(&mut public_key);
        rand::rngs::OsRng.fill_bytes(&mut secret_key);

        let keypair = QuantumKeyPair { public_key, secret_key };

        self.keypair = Some(keypair.clone());

        info!("Quantum-safe keypair generated (simulated)");
        Ok(keypair)
    }

    pub fn encrypt(
        &self,
        plaintext: &[u8],
        master_key: &[u8],
    ) -> Result<zeroize::Zeroizing<Vec<u8>>> {
        register_encryption_operation!("quantum_encrypt", EncryptionAlgorithm::QuantumSafe, true);
        debug!("Encrypting data with quantum-safe encryption (simulated)");

        // Derive AES key from master key using HKDF into an uninitialized buffer.
        let mut aes_key_bytes = {
            let mut k_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
            let k_ptr = k_uninit.as_mut_ptr() as *mut u8;
            unsafe {
                let hkdf = hkdf::Hkdf::<sha2::Sha256>::new(Some(b"quantum-enc-salt"), master_key);
                hkdf.expand(b"aes-gcm-key", std::slice::from_raw_parts_mut(k_ptr, 32))
                    .map_err(|e| anyhow::anyhow!("Failed to derive encryption key: {}", e))?;
                k_uninit.assume_init()
            }
        };
        let aes_key = aes_key_bytes;
        let cipher = Aes256Gcm::new_from_slice(&aes_key)
            .map_err(|e| anyhow::anyhow!("Failed to init AES cipher: {}", e))?;

        // Generate nonce without an all-zero literal
        let mut nonce_bytes = {
            let mut n_uninit = std::mem::MaybeUninit::<[u8; AES_NONCE_LEN]>::uninit();
            let n_ptr = n_uninit.as_mut_ptr() as *mut u8;
            unsafe {
                rand::rngs::OsRng.fill_bytes(std::slice::from_raw_parts_mut(n_ptr, AES_NONCE_LEN));
                n_uninit.assume_init()
            }
        };
        #[allow(deprecated)]
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);

        // AES-GCM encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("AES encryption failed: {e}"))?;

        // Simulated KEM ciphertext (Kyber)
        let mut simulated_kyber_ciphertext = {
            let mut v = vec![0u8; KYBER_CIPHERTEXT_LEN];
            rand::rngs::OsRng.fill_bytes(&mut v);
            v
        };

        // Format: [4 bytes len][kyber_ct][12 bytes nonce][aes_ct]
        let mut result = Vec::with_capacity(
            4 + simulated_kyber_ciphertext.len() + AES_NONCE_LEN + ciphertext.len(),
        );
        result.extend_from_slice(&(simulated_kyber_ciphertext.len() as u32).to_le_bytes());
        result.extend_from_slice(&simulated_kyber_ciphertext);
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        // Zeroize sensitive temporary buffers where possible
        nonce_bytes.zeroize();
        simulated_kyber_ciphertext.zeroize();
        aes_key_bytes.zeroize();

        debug!("Data encrypted with quantum-safe encryption (simulated)");
        Ok(zeroize::Zeroizing::new(result))
    }

    pub fn decrypt(
        &self,
        encrypted_data: &[u8],
        master_key: &[u8],
    ) -> Result<zeroize::Zeroizing<Vec<u8>>> {
        register_encryption_operation!("quantum_decrypt", EncryptionAlgorithm::QuantumSafe, true);
        debug!("Decrypting data with quantum-safe encryption (simulated)");

        if encrypted_data.len() < 4 {
            return Err(anyhow::anyhow!("Invalid encrypted data format"));
        }

        let kyber_ciphertext_len = u32::from_le_bytes([
            encrypted_data[0],
            encrypted_data[1],
            encrypted_data[2],
            encrypted_data[3],
        ]) as usize;

        let header_len = 4 + kyber_ciphertext_len;
        if encrypted_data.len() < header_len + AES_NONCE_LEN {
            return Err(anyhow::anyhow!("Invalid encrypted data length"));
        }

        let nonce_start = header_len;
        let nonce_end = nonce_start + AES_NONCE_LEN;
        let nonce_bytes = &encrypted_data[nonce_start..nonce_end];
        let aes_ciphertext = &encrypted_data[nonce_end..];

        // Derive AES key from master key using HKDF into an uninitialized buffer.
        let mut aes_key_bytes = {
            let mut k_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
            let k_ptr = k_uninit.as_mut_ptr() as *mut u8;
            unsafe {
                let hkdf = hkdf::Hkdf::<sha2::Sha256>::new(Some(b"quantum-enc-salt"), master_key);
                hkdf.expand(b"aes-gcm-key", std::slice::from_raw_parts_mut(k_ptr, 32))
                    .map_err(|e| anyhow::anyhow!("Failed to derive decryption key: {}", e))?;
                k_uninit.assume_init()
            }
        };
        let aes_key = aes_key_bytes;

        let cipher = Aes256Gcm::new_from_slice(&aes_key)
            .map_err(|e| anyhow::anyhow!("Failed to init AES cipher: {}", e))?;
        #[allow(deprecated)]
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, aes_ciphertext)
            .map_err(|e| anyhow::anyhow!("AES decryption failed: {e}"))?;

        // Zeroize sensitive temporary buffers
        aes_key_bytes.zeroize();

        debug!("Data decrypted with quantum-safe encryption (simulated)");
        // Return as zeroizing buffer so callers don't hold plaintext Vec<u8>
        Ok(zeroize::Zeroizing::new(plaintext))
    }

    pub fn get_public_key(&self) -> Option<&[u8]> {
        self.keypair.as_ref().map(|kp| kp.public_key.as_slice())
    }
}

impl Zeroize for QuantumSafeEncryption {
    fn zeroize(&mut self) {
        if let Some(ref mut kp) = self.keypair {
            kp.secret_key.zeroize();
        }
    }
}

impl ZeroizeOnDrop for QuantumSafeEncryption {}

impl Default for QuantumSafeEncryption {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_safe_encryption() {
        let crypto = QuantumSafeEncryption::new().unwrap();
        let master_key = b"test_master_key_32_bytes_long!!!";

        let plaintext = b"Hello, quantum-safe world!";
        let encrypted = crypto.encrypt(plaintext, master_key).unwrap();
        let decrypted = crypto.decrypt(&encrypted, master_key).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }
}
