use anyhow::Result;
use hkdf::Hkdf;
use pbkdf2::pbkdf2_hmac;
use scrypt::Params;
use sha2::Sha256;
use tracing::{debug, info};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Clone, PartialEq)]
pub enum KDFAlgorithm {
    PBKDF2 { iterations: u32 },
    Scrypt { n: u32, r: u32, p: u32 },
    HKDF,
}
pub struct KeyDerivation {
    algorithm: KDFAlgorithm,
}

impl KeyDerivation {
    pub fn new(algorithm: KDFAlgorithm) -> Self {
        info!("馃攽 Initializing Key Derivation with algorithm: {:?}", algorithm);
        Self { algorithm }
    }

    pub fn pbkdf2(iterations: u32) -> Self {
        Self::new(KDFAlgorithm::PBKDF2 { iterations })
    }

    pub fn scrypt(n: u32, r: u32, p: u32) -> Self {
        Self::new(KDFAlgorithm::Scrypt { n, r, p })
    }

    pub fn hkdf() -> Self {
        Self::new(KDFAlgorithm::HKDF)
    }

    pub fn derive_key(
        &self,
        password: &[u8],
        salt: &[u8],
        key_length: usize,
    ) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Deriving key with length {} bytes", key_length);

        match &self.algorithm {
            KDFAlgorithm::PBKDF2 { iterations } => {
                self.derive_pbkdf2(password, salt, *iterations, key_length)
            }
            KDFAlgorithm::Scrypt { n, r, p } => {
                self.derive_scrypt(password, salt, *n, *r, *p, key_length)
            }
            KDFAlgorithm::HKDF => self.derive_hkdf(password, salt, key_length),
        }
    }

    fn derive_pbkdf2(
        &self,
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        key_length: usize,
    ) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Using PBKDF2 with {} iterations", iterations);

        let mut key = Zeroizing::new(vec![0u8; key_length]);
        pbkdf2_hmac::<Sha256>(password, salt, iterations, &mut key);

        debug!("鉁?PBKDF2 key derived successfully");
        Ok(key)
    }

    fn derive_scrypt(
        &self,
        password: &[u8],
        salt: &[u8],
        n: u32,
        r: u32,
        p: u32,
        key_length: usize,
    ) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Using Scrypt with parameters N={}, r={}, p={}", n, r, p);

        let params = Params::new((n as f64).log2() as u8, r, p, key_length)
            .map_err(|e| anyhow::anyhow!("Invalid Scrypt parameters: {}", e))?;

        let mut key = Zeroizing::new(vec![0u8; key_length]);
        scrypt::scrypt(password, salt, &params, &mut key)
            .map_err(|e| anyhow::anyhow!("Scrypt derivation failed: {}", e))?;

        debug!("鉁?Scrypt key derived successfully");
        Ok(key)
    }

    fn derive_hkdf(
        &self,
        input_key_material: &[u8],
        salt: &[u8],
        key_length: usize,
    ) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Using HKDF key derivation");

        let hk = Hkdf::<Sha256>::new(Some(salt), input_key_material);
        let mut key = Zeroizing::new(vec![0u8; key_length]);

        hk.expand(b"defi-wallet-key", &mut key)
            .map_err(|e| anyhow::anyhow!("HKDF expansion failed: {}", e))?;

        debug!("鉁?HKDF key derived successfully");
        Ok(key)
    }

    pub fn generate_salt(length: usize) -> Vec<u8> {
        use rand::RngCore;
        let mut salt = vec![0u8; length];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        salt
    }

    pub fn derive_key_from_mnemonic(
        &self,
        mnemonic: &str,
        passphrase: &str,
        salt: &[u8],
        key_length: usize,
    ) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Deriving key from mnemonic phrase");

        // Combine mnemonic and passphrase
        let mut combined_input = mnemonic.as_bytes().to_vec();
        combined_input.extend_from_slice(passphrase.as_bytes());

        let key = self.derive_key(&combined_input, salt, key_length)?;

        // Clear the combined input
        combined_input.zeroize();

        debug!("鉁?Key derived from mnemonic successfully");
        Ok(key)
    }

    pub fn derive_child_key(
        &self,
        parent_key: &[u8],
        index: u32,
        chain_code: &[u8],
        key_length: usize,
    ) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Deriving child key with index: {}", index);

        // Simplified child key derivation (BIP32-like)
        let mut input = parent_key.to_vec();
        input.extend_from_slice(&index.to_be_bytes());

        let child_key = self.derive_key(&input, chain_code, key_length)?;

        // Zeroize temporary buffer containing sensitive material
        input.zeroize();

        debug!("鉁?Child key derived successfully");
        Ok(child_key)
    }

    pub fn strengthen_key(&self, weak_key: &[u8], salt: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Strengthening weak key material");

        // Use a high iteration count for strengthening
        let strong_kdf = KeyDerivation::pbkdf2(100_000);
        let strengthened = strong_kdf.derive_key(weak_key, salt, 32)?;

        debug!("鉁?Key strengthened successfully");
        Ok(strengthened)
    }
}

impl Default for KeyDerivation {
    fn default() -> Self {
        // Default to Scrypt with secure parameters (OWASP recommendation: N=2^17, r=8, p=1)
        // N must be a power of 2. 2^17 = 131072.
        Self::scrypt(131072, 8, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbkdf2_derivation() {
        let kdf = KeyDerivation::pbkdf2(10000);
        let password = b"test_password";
        let salt = b"test_salt_123";

        let key1 = kdf.derive_key(password, salt, 32).unwrap();
        let key2 = kdf.derive_key(password, salt, 32).unwrap();

        // Same inputs should produce same key
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);

        // Different salt should produce different key
        let salt2 = b"different_salt";
        let key3 = kdf.derive_key(password, salt2, 32).unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_scrypt_derivation() {
        let kdf = KeyDerivation::scrypt(16384, 8, 1);
        let password = b"test_password";
        let salt = b"test_salt_123";

        let key = kdf.derive_key(password, salt, 32).unwrap();
        assert_eq!(key.len(), 32);

        // Verify deterministic behavior
        let key2 = kdf.derive_key(password, salt, 32).unwrap();
        assert_eq!(key, key2);
    }

    #[test]
    fn test_hkdf_derivation() {
        let kdf = KeyDerivation::hkdf();
        let ikm = b"input_key_material";
        let salt = b"optional_salt";

        let key = kdf.derive_key(ikm, salt, 32).unwrap();
        assert_eq!(key.len(), 32);

        // Verify deterministic behavior
        let key2 = kdf.derive_key(ikm, salt, 32).unwrap();
        assert_eq!(key, key2);
    }

    #[test]
    fn test_mnemonic_key_derivation() {
        let kdf = KeyDerivation::default();
        let mnemonic =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let passphrase = "TREZOR";
        let salt = KeyDerivation::generate_salt(16);

        let key = kdf.derive_key_from_mnemonic(mnemonic, passphrase, &salt, 64).unwrap();
        assert_eq!(key.len(), 64);
    }

    #[test]
    fn test_child_key_derivation() {
        let kdf = KeyDerivation::default();
        let parent_key = &[0u8; 32];
        let chain_code = &[1u8; 32];

        let child1 = kdf.derive_child_key(parent_key, 0, chain_code, 32).unwrap();
        let child2 = kdf.derive_child_key(parent_key, 1, chain_code, 32).unwrap();

        assert_eq!(child1.len(), 32);
        assert_eq!(child2.len(), 32);
        assert_ne!(child1, child2); // Different indices should produce different keys
    }

    #[test]
    fn test_salt_generation() {
        let salt1 = KeyDerivation::generate_salt(16);
        let salt2 = KeyDerivation::generate_salt(16);

        assert_eq!(salt1.len(), 16);
        assert_eq!(salt2.len(), 16);
        assert_ne!(salt1, salt2); // Should be random
    }
}
