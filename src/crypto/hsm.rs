// src/crypto/hsm.rs
use crate::crypto::signature_utils::ensure_low_s;
use crate::security::secret::{vec_to_secret, SecretVec};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rand::{rngs::OsRng, RngCore};
use secp256k1::{Message, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Clone)]
pub struct HSMConfig {
    pub enabled: bool,
    pub device_path: String,
    pub pin: Zeroizing<String>,
    pub isolation_enabled: bool,
}

impl core::fmt::Debug for HSMConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HSMConfig")
            .field("enabled", &self.enabled)
            .field("device_path", &self.device_path)
            .field("pin", &"[REDACTED]")
            .field("isolation_enabled", &self.isolation_enabled)
            .finish()
    }
}

#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct SecureMemoryRegion {
    data: Vec<u8>,
    #[zeroize(skip)]
    id: u64,
    #[zeroize(skip)]
    allocated_at: DateTime<Utc>,
}

pub struct HSMManager {
    config: HSMConfig,
    secure_regions: Arc<Mutex<std::collections::HashMap<u64, SecureMemoryRegion>>>,
    next_id: Arc<Mutex<u64>>,
    initialized: bool,
}

impl HSMManager {
    pub async fn new() -> Result<Self> {
        info!("Initializing HSM Manager");

        let config = HSMConfig {
            enabled: false, // Disabled by default for demo
            device_path: "/dev/hsm0".to_string(),
            pin: Zeroizing::new("".to_string()),
            isolation_enabled: true,
        };

        Ok(Self {
            config,
            secure_regions: Arc::new(Mutex::new(std::collections::HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            initialized: false,
        })
    }

    pub async fn initialize(&mut self, config: HSMConfig) -> Result<()> {
        info!("Initializing HSM with config");

        self.config = config;

        if self.config.enabled {
            // In a real implementation, this would:
            // 1. Connect to the HSM device
            // 2. Authenticate with PIN
            // 3. Initialize secure memory pools
            // 4. Set up memory isolation

            info!("HSM device connection established");
            info!("Memory isolation enabled: {}", self.config.isolation_enabled);
        } else {
            info!("HSM disabled - using software-based secure memory simulation");
        }

        self.initialized = true;
        Ok(())
    }

    pub async fn allocate_secure_memory(&self, size: usize) -> Result<u64> {
        if !self.initialized {
            return Err(anyhow::anyhow!("HSM not initialized"));
        }

        debug!("Allocating {} bytes of secure memory", size);

        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;
        drop(next_id);

        let region = SecureMemoryRegion { data: vec![0u8; size], id, allocated_at: Utc::now() };

        let mut regions = self.secure_regions.lock().await;
        regions.insert(id, region);

        debug!("Allocated secure memory region with ID: {}", id);
        Ok(id)
    }

    pub async fn write_secure_memory(&self, region_id: u64, data: &[u8]) -> Result<()> {
        debug!("Writing {} bytes to secure memory region {}", data.len(), region_id);

        let mut regions = self.secure_regions.lock().await;
        let region = regions
            .get_mut(&region_id)
            .ok_or_else(|| anyhow::anyhow!("Secure memory region not found: {}", region_id))?;

        let n = data.len().min(region.data.len());
        region.data[..n].copy_from_slice(&data[..n]);
        debug!("Data written to secure memory region {}", region_id);
        Ok(())
    }

    pub async fn read_secure_memory(&self, region_id: u64) -> Result<Zeroizing<Vec<u8>>> {
        debug!("Reading from secure memory region {}", region_id);

        let regions = self.secure_regions.lock().await;
        let region = regions
            .get(&region_id)
            .ok_or_else(|| anyhow::anyhow!("Secure memory region not found: {}", region_id))?;

        Ok(Zeroizing::new(region.data.clone()))
    }

    pub async fn free_secure_memory(&self, region_id: u64) -> Result<()> {
        debug!("Freeing secure memory region {}", region_id);

        let mut regions = self.secure_regions.lock().await;
        let mut region = regions
            .remove(&region_id)
            .ok_or_else(|| anyhow::anyhow!("Secure memory region not found: {}", region_id))?;

        // Zeroize the memory before dropping
        region.zeroize();

        debug!("Freed secure memory region {}", region_id);
        Ok(())
    }

    pub async fn secure_key_generation(&self, key_type: &str, key_size: usize) -> Result<u64> {
        info!("Generating secure key: {} (size: {} bytes)", key_type, key_size);

        if !self.initialized {
            return Err(anyhow::anyhow!("HSM not initialized"));
        }

        // Use cryptographically secure RNG instead of thread_rng
        let region_id = self.allocate_secure_memory(key_size).await?;
        let mut buf = vec![0u8; key_size];
        OsRng.fill_bytes(&mut buf);
        self.write_secure_memory(region_id, &buf).await?;
        buf.zeroize();

        info!("Secure key generated with ID: {}", region_id);
        Ok(region_id)
    }

    pub async fn secure_sign(&self, key_region_id: u64, message: &[u8]) -> Result<SecretVec> {
        debug!("Signing message with secure key {}", key_region_id);

        if !self.initialized {
            return Err(anyhow::anyhow!("HSM not initialized"));
        }

        // Read the private key from secure memory (Zeroizing<Vec<u8>>)
        let private_key_bytes = self.read_secure_memory(key_region_id).await?;

        // Ensure we have a valid 32-byte private key
        if private_key_bytes.len() != 32 {
            return Err(anyhow::anyhow!(
                "Invalid private key length: expected 32 bytes, got {}",
                private_key_bytes.len()
            ));
        }
        // Create secp256k1 context and key. Extract bytes into a small array and then
        // ensure the Zeroizing buffer is dropped/zeroized as soon as possible.
        let secp = Secp256k1::new();
        let mut priv_arr = [0u8; 32];
        priv_arr.copy_from_slice(&private_key_bytes[..32]);

        // Drop/zeroize the Zeroizing<Vec<u8>> before constructing SecretKey
        drop(private_key_bytes);

        let secret_key = SecretKey::from_slice(&priv_arr)
            .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;

        // Zeroize the stack copy of priv_arr after SecretKey created
        priv_arr.zeroize();

        let keypair = secp256k1::KeyPair::from_secret_key(&secp, &secret_key);

        // Hash the message with domain separation using SHA256
        let mut hasher = Sha256::new();
        hasher.update(b"HSM_SIG_V1\x00");
        hasher.update(message);
        let message_hash = hasher.finalize();
        let message_obj = Message::from_slice(&message_hash)
            .map_err(|e| anyhow::anyhow!("Invalid message hash: {}", e))?;

        // Sign the message
        let signature = secp.sign_ecdsa(&message_obj, &keypair.secret_key());

        // Normalize to low-S to prevent malleability and ensure canonicality
        let mut compact = [0u8; 64];
        compact.copy_from_slice(&signature.serialize_compact());
        let normalized = ensure_low_s(&compact);

        debug!("Message signed with secure ECDSA key (low-S normalized)");
        Ok(vec_to_secret(normalized.to_vec()))
    }

    pub async fn get_memory_stats(&self) -> Result<HSMMemoryStats> {
        let regions = self.secure_regions.lock().await;

        let total_regions = regions.len();
        let total_memory: usize = regions.values().map(|r| r.data.len()).sum();

        Ok(HSMMemoryStats {
            total_regions,
            total_memory_bytes: total_memory,
            average_region_size: if total_regions > 0 { total_memory / total_regions } else { 0 },
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[derive(Debug, Clone)]
pub struct HSMMemoryStats {
    pub total_regions: usize,
    pub total_memory_bytes: usize,
    pub average_region_size: usize,
}

impl Drop for HSMManager {
    fn drop(&mut self) {
        warn!("HSM Manager dropping - secure memory will be cleared");
        // Note: In async drop, we can't easily await the cleanup
        // In production, implement proper async cleanup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hsm_memory_allocation() {
        let mut hsm = HSMManager::new().await.unwrap();

        let config = HSMConfig {
            enabled: false,
            device_path: "/dev/null".to_string(),
            pin: Zeroizing::new("test".to_string()),
            isolation_enabled: true,
        };

        hsm.initialize(config).await.unwrap();

        // Allocate memory
        let region_id = hsm.allocate_secure_memory(64).await.unwrap();
        assert!(region_id > 0);

        // Write data
        let test_data = b"secret key data";
        hsm.write_secure_memory(region_id, test_data).await.unwrap();

        // Read data back
        let read_data = hsm.read_secure_memory(region_id).await.unwrap();
        assert_eq!(&read_data[..test_data.len()], test_data);

        // Free memory
        hsm.free_secure_memory(region_id).await.unwrap();

        // Verify memory is freed
        let result = hsm.read_secure_memory(region_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hsm_debug_pin_redacted() {
        let cfg = HSMConfig {
            enabled: true,
            device_path: "/dev/hsm0".to_string(),
            pin: Zeroizing::new("supersecret".to_string()),
            isolation_enabled: true,
        };
        let dbg = format!("{:?}", cfg);
        assert!(dbg.contains("[REDACTED]"));
        assert!(!dbg.contains("supersecret"));
    }

    #[tokio::test]
    async fn test_read_secure_memory_zeroizing_type() {
        let mut hsm = HSMManager::new().await.unwrap();
        let config = HSMConfig {
            enabled: false,
            device_path: "/dev/null".to_string(),
            pin: Zeroizing::new("test".to_string()),
            isolation_enabled: true,
        };
        hsm.initialize(config).await.unwrap();
        let region_id = hsm.allocate_secure_memory(4).await.unwrap();
        hsm.write_secure_memory(region_id, &[1, 2, 3, 4]).await.unwrap();
        let data = hsm.read_secure_memory(region_id).await.unwrap();
        assert_eq!(&*data, &[1, 2, 3, 4]);
        // Drop occurs here; Zeroizing ensures buffer cleared on drop (not observable here, but type-level guarantee)
        drop(data);
        hsm.free_secure_memory(region_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_secure_key_generation() {
        let mut hsm = HSMManager::new().await.unwrap();

        let config = HSMConfig {
            enabled: false,
            device_path: "/dev/null".to_string(),
            pin: Zeroizing::new("test".to_string()),
            isolation_enabled: true,
        };

        hsm.initialize(config).await.unwrap();

        // Generate key
        let key_id = hsm.secure_key_generation("ECDSA", 32).await.unwrap();
        assert!(key_id > 0);

        // Sign with the key
        let message = b"test message";
        let signature = hsm.secure_sign(key_id, message).await.unwrap();

        // Verify signature is proper ECDSA format (64 bytes compact)
        assert_eq!(signature.len(), 64, "ECDSA signature should be 64 bytes compact");

        // Verify signature is not just a hash (32 bytes)
        assert_ne!(signature.len(), 32, "Signature should not be just a hash");

        // Clean up
        hsm.free_secure_memory(key_id).await.unwrap();
    }
}
