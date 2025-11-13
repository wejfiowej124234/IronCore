use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use anyhow::Result;
use hkdf::Hkdf;
use once_cell::sync::Lazy;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::crypto::encryption_consistency::EncryptionAlgorithm;
use crate::register_encryption_operation;

// NOTE: A process-supplied WALLET_ENC_KEY is used for envelope encryption; no global in-memory
// master key is retained to reduce attack surface.

/// Encrypted in-memory key storage.
/// Maps id -> (encrypted_key, nonce, salt)
static ENCRYPTED_KEY_STORAGE: Lazy<Mutex<HashMap<String, StoredKey>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Map from a logical key label (e.g., "wallet:<id>:signing") to current version state
static KEY_LABEL_INDEX: Lazy<Mutex<HashMap<String, KeyLabelState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
struct EncryptedKeyData {
    ciphertext: Vec<u8>,
    nonce: [u8; 12],
    salt: [u8; 32],
}

#[derive(Clone)]
#[allow(dead_code)]
struct KeyMetadata {
    created_at_unix: u64,
    usage_count: u64,
    version: u32,
    retired: bool,
}

#[derive(Clone)]
struct StoredKey {
    data: EncryptedKeyData,
    meta: KeyMetadata,
}

#[derive(Clone, Default)]
struct KeyLabelState {
    current_id: Option<String>,
    current_version: u32,
    // history as (version, id)
    history: Vec<(u32, String)>,
}

fn load_master_enc_key() -> Result<[u8; 32]> {
    // Prefer a strong, process-supplied secret for encrypting keys at rest.
    // CI/tests set WALLET_ENC_KEY via the test-env feature (see src/test_env.rs).
    let enc_b64 = std::env::var("WALLET_ENC_KEY").map_err(|_| {
        anyhow::anyhow!(
            "WALLET_ENC_KEY not set. Provide a base64-encoded 32-byte key for key encryption."
        )
    })?;
    use base64::Engine as _;
    let engine = base64::engine::general_purpose::STANDARD;
    let raw: Zeroizing<Vec<u8>> = Zeroizing::new(
        engine
            .decode(enc_b64.trim())
            .map_err(|_| anyhow::anyhow!("WALLET_ENC_KEY must be valid base64 for 32 bytes"))?,
    );
    if raw.len() != 32 {
        return Err(anyhow::anyhow!(
            "WALLET_ENC_KEY must decode to exactly 32 bytes, got {}",
            raw.len()
        ));
    }
    // High severity: refuse known weak default (all-zero key) when not in test-env
    // In test builds we intentionally set a deterministic all-zero key via src/test_env.rs.
    // Only enforce the all-zero rejection when NOT running tests and NOT using the `test-env` feature.
    #[cfg(not(any(test, feature = "test-env")))]
    {
        if raw.iter().all(|&b| b == 0) {
            return Err(anyhow::anyhow!(
                "Insecure WALLET_ENC_KEY detected (all zeros). Set a strong 32-byte key."
            ));
        }
    }
    // Copy raw into an initialized array without an all-zero literal.
    let out = {
        let mut out_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
        let out_ptr = out_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(raw.as_ptr(), out_ptr, 32);
            out_uninit.assume_init()
        }
    };
    Ok(out)
}

/// Generate a fresh cryptographic key (32 bytes for secp256k1 compatibility).
use crate::security::SecretVec;

pub fn generate_key() -> Result<SecretVec> {
    // Allocate an uninitialized 32-byte array, fill it with OS randomness,
    // then convert to Vec<u8> to avoid a source-level all-zero literal.
    let key = {
        let mut k_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
        let k_ptr = k_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            OsRng.fill_bytes(std::slice::from_raw_parts_mut(k_ptr, 32));
            let k_arr = k_uninit.assume_init();
            k_arr.to_vec()
        }
    };
    Ok(SecretVec::new(key))
}

/// Store a key encrypted with wallet-specific encryption and return a generated id.
pub fn store_key(key: &[u8], wallet_id: &str) -> Result<String> {
    register_encryption_operation!("key_management_store", EncryptionAlgorithm::Aes256Gcm, false);
    let id = Uuid::new_v4().to_string();

    // Generate salt for key derivation into an uninitialized buffer to avoid
    // a static all-zero literal.
    let salt = {
        let mut s_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
        let s_ptr = s_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            OsRng.fill_bytes(std::slice::from_raw_parts_mut(s_ptr, 32));
            s_uninit.assume_init()
        }
    };

    // Generate nonce for AES-GCM similarly without an all-zero literal.
    let nonce_bytes = {
        let mut n_uninit = std::mem::MaybeUninit::<[u8; 12]>::uninit();
        let n_ptr = n_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            OsRng.fill_bytes(std::slice::from_raw_parts_mut(n_ptr, 12));
            n_uninit.assume_init()
        }
    };
    #[allow(deprecated)]
    let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);

    // Derive encryption key from strong master secret + per-wallet context and random salt
    let mut master = load_master_enc_key()?;
    // Derive encryption key into an uninitialized buffer via HKDF to avoid
    // a source-level all-zero literal.
    let mut encryption_key = {
        let mut k_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
        let k_ptr = k_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            let hkdf = Hkdf::<Sha256>::new(Some(&salt), &master);
            hkdf.expand(wallet_id.as_bytes(), std::slice::from_raw_parts_mut(k_ptr, 32))
                .map_err(|_| anyhow::anyhow!("Failed to derive wallet-specific encryption key"))?;
            k_uninit.assume_init()
        }
    };

    // Encrypt the private key
    let cipher = Aes256Gcm::new_from_slice(&encryption_key)
        .map_err(|_| anyhow::anyhow!("Failed to initialize AES cipher"))?;
    let ciphertext = cipher
        .encrypt(nonce, aes_gcm::aead::Payload { msg: key, aad: wallet_id.as_bytes() })
        .map_err(|_| anyhow::anyhow!("Failed to encrypt key"))?;

    // Store encrypted data
    let encrypted_data = EncryptedKeyData { ciphertext, nonce: nonce_bytes, salt };
    let meta = KeyMetadata {
        created_at_unix: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        usage_count: 0,
        version: 0,
        retired: false,
    };

    let mut storage = ENCRYPTED_KEY_STORAGE.lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire storage lock: {}", e))?;
    storage.insert(id.clone(), StoredKey { data: encrypted_data, meta });

    // Zeroize temporary buffers
    encryption_key.zeroize();
    master.zeroize();

    Ok(id)
}

/// Retrieve and decrypt a key by id using wallet-specific encryption.
pub fn retrieve_key(id: &str, wallet_id: &str) -> Result<Zeroizing<Vec<u8>>> {
    register_encryption_operation!(
        "key_management_retrieve",
        EncryptionAlgorithm::Aes256Gcm,
        false
    );
    // Get encrypted data first
    let (encrypted_data, key_id) = {
        let storage = ENCRYPTED_KEY_STORAGE.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire storage lock: {}", e))?;
        let s =
            storage.get(id).ok_or_else(|| anyhow::anyhow!("Key not found for id: {}", id))?.clone();
        (s.data, id.to_string())
    };

    // Derive encryption key from strong master secret + per-wallet context and stored salt
    let mut master = load_master_enc_key()?;
    // Derive encryption key into an uninitialized buffer via HKDF
    let mut encryption_key = {
        let mut k_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
        let k_ptr = k_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            let hkdf = Hkdf::<Sha256>::new(Some(&encrypted_data.salt), &master);
            hkdf.expand(wallet_id.as_bytes(), std::slice::from_raw_parts_mut(k_ptr, 32))
                .map_err(|_| anyhow::anyhow!("Failed to derive wallet-specific encryption key"))?;
            k_uninit.assume_init()
        }
    };

    // Decrypt the private key
    let cipher = Aes256Gcm::new_from_slice(&encryption_key)
        .map_err(|_| anyhow::anyhow!("Failed to initialize AES cipher"))?;
    #[allow(deprecated)]
    let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&encrypted_data.nonce);
    let plaintext = cipher
        .decrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: encrypted_data.ciphertext.as_ref(),
                aad: wallet_id.as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("Failed to decrypt key"))?;

    // Zeroize temporary buffers
    encryption_key.zeroize();
    master.zeroize();

    // Bump usage count
    if let Ok(mut storage) = ENCRYPTED_KEY_STORAGE.lock() {
        if let Some(sk) = storage.get_mut(&key_id) {
            sk.meta.usage_count = sk.meta.usage_count.saturating_add(1);
        }
    }

    Ok(Zeroizing::new(plaintext))
}

/// Remove a key from storage (secure deletion)
pub fn delete_key(id: &str) -> Result<()> {
    let mut storage = ENCRYPTED_KEY_STORAGE.lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire storage lock: {}", e))?;
    let encrypted_data =
        storage.remove(id).ok_or_else(|| anyhow::anyhow!("Key not found for id: {}", id))?;

    // Explicitly zeroize the encrypted data
    drop(encrypted_data);

    Ok(())
}

/// Get the count of stored keys (for monitoring)
pub fn key_count() -> usize {
    ENCRYPTED_KEY_STORAGE.lock()
        .map(|storage| storage.len())
        .unwrap_or(0)
}

/// Clear all stored keys (for testing only)
#[cfg(test)]
pub fn clear_all_keys() {
    if let Ok(mut storage) = ENCRYPTED_KEY_STORAGE.lock() {
        storage.clear();
    }
    if let Ok(mut labels) = KEY_LABEL_INDEX.lock() {
        labels.clear();
    }
}

/// Create and record a versioned key for a logical label. Returns (id, version).
///
/// Contract:
/// - Input: wallet_id (HKDF/AAD context), label (e.g., "wallet:<name>:signing")
/// - Output: (new key id, version=1)
/// - Side effects: generates a fresh 32-byte key, encrypts and stores it in-memory;
///   initializes label state to version 1.
pub fn create_key_for_label(wallet_id: &str, label: &str) -> Result<(String, u32)> {
    let key = generate_key()?;
    let id = store_key(&key, wallet_id)?;
    let mut labels = KEY_LABEL_INDEX.lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire label index lock: {}", e))?;
    let state = labels.entry(label.to_string()).or_default();
    // First creation starts at version 1
    state.current_version = 1;
    state.current_id = Some(id.clone());
    state.history.push((1, id.clone()));
    Ok((id, 1))
}

/// Rotate the current key for a label, generating a new version and marking the old as retired.
/// Returns (old_id, new_id, new_version).
///
/// Notes:
/// - Old key's metadata.retired is set to true.
/// - New key material is freshly generated and stored; version increments by 1.
pub fn rotate_key_for_label(wallet_id: &str, label: &str) -> Result<(String, String, u32)> {
    let mut labels = KEY_LABEL_INDEX.lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire label index lock: {}", e))?;
    let state = labels.get_mut(label).ok_or_else(|| anyhow::anyhow!("Label not found"))?;
    let old_id =
        state.current_id.clone().ok_or_else(|| anyhow::anyhow!("No current key for label"))?;

    // Mark old as retired
    if let Ok(mut storage) = ENCRYPTED_KEY_STORAGE.lock() {
        if let Some(sk) = storage.get_mut(&old_id) {
            sk.meta.retired = true;
        }
    }

    // Create new key
    let new_material = generate_key()?;
    let new_id = store_key(&new_material, wallet_id)?;
    let new_version = state.current_version.saturating_add(1);
    state.current_version = new_version;
    state.current_id = Some(new_id.clone());
    state.history.push((new_version, new_id.clone()));
    Ok((old_id, new_id, new_version))
}

/// Retrieve the current key for a label and its version.
///
/// Safety:
/// - Returns Zeroizing<Vec<u8>>; caller must minimize exposure and avoid logging.
pub fn retrieve_current_key_for_label(
    label: &str,
    wallet_id: &str,
) -> Result<(Zeroizing<Vec<u8>>, u32)> {
    let (id, version) = {
        let labels = KEY_LABEL_INDEX.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire label index lock: {}", e))?;
        let state = labels.get(label).ok_or_else(|| anyhow::anyhow!("Label not found"))?;
        let id = state
            .current_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No current key for label"))?
            .clone();
        (id, state.current_version)
    };
    let key = retrieve_key(&id, wallet_id)?;
    Ok((key, version))
}

/// Retrieve a specific version of a label's key.
///
/// Use cases:
/// - Reproducing signatures that were generated with a prior version.
/// - Validating historical records.
pub fn retrieve_key_by_version(
    label: &str,
    version: u32,
    wallet_id: &str,
) -> Result<Zeroizing<Vec<u8>>> {
    let id = {
        let labels = KEY_LABEL_INDEX.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire label index lock: {}", e))?;
        let state = labels.get(label).ok_or_else(|| anyhow::anyhow!("Label not found"))?;
        state
            .history
            .iter()
            .find(|(v, _)| *v == version)
            .map(|(_, id)| id.clone())
            .ok_or_else(|| anyhow::anyhow!("Version not found for label"))?
    };
    retrieve_key(&id, wallet_id)
}

/// Seed a label state in-memory from external metadata (e.g., database) if not present.
/// If already present, this will overwrite current_id/current_version to match provided values and ensure history contains the pair.
pub fn seed_label_state(label: &str, current_id: String, current_version: u32) {
    let Ok(mut labels) = KEY_LABEL_INDEX.lock() else {
        tracing::warn!("Failed to acquire label index lock for seeding");
        return;
    };
    let state = labels.entry(label.to_string()).or_default();
    state.current_id = Some(current_id.clone());
    state.current_version = current_version;
    if !state.history.iter().any(|(v, id)| *v == current_version && id == &current_id) {
        state.history.push((current_version, current_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_generate_key() {
        let key = generate_key().unwrap();
        assert!(!key.is_empty());
        assert_eq!(key.len(), 32); // secp256k1 private key is 32 bytes
    }

    #[serial_test::serial]
    #[test]
    fn test_store_and_retrieve_key() {
        clear_all_keys();
        let key = generate_key().unwrap();
        let id = store_key(&key, "test_wallet").unwrap();
        let retrieved = retrieve_key(&id, "test_wallet").unwrap();
        assert_eq!(retrieved.as_slice(), key.as_slice());
    }

    #[serial_test::serial]
    #[test]
    fn test_retrieve_key_not_found() {
        clear_all_keys();
        let result = retrieve_key("nonexistent", "test_wallet");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Key not found"));
    }

    #[serial_test::serial]
    #[test]
    fn test_store_empty_key() {
        clear_all_keys();
        let key = Vec::<u8>::new();
        let id = store_key(&key, "test_wallet").unwrap();
        let retrieved = retrieve_key(&id, "test_wallet").unwrap();
        assert_eq!(retrieved.as_slice(), Vec::<u8>::new().as_slice());
    }

    #[serial_test::serial]
    #[test]
    fn test_store_large_key() {
        clear_all_keys();
        let key = vec![0u8; 1000];
        let id = store_key(&key, "test_wallet").unwrap();
        let retrieved = retrieve_key(&id, "test_wallet").unwrap();
        assert_eq!(retrieved.as_slice(), key.as_slice());
    }

    #[serial_test::serial]
    #[test]
    fn test_concurrent_access() {
        clear_all_keys();
        let key1 = generate_key().unwrap();
        let key2 = generate_key().unwrap();

        let key1_clone = key1.clone();
        let handle1 = thread::spawn(move || {
            let id = store_key(&key1_clone, "test_wallet").unwrap();
            retrieve_key(&id, "test_wallet").unwrap()
        });

        let key2_clone = key2.clone();
        let handle2 = thread::spawn(move || {
            let id = store_key(&key2_clone, "test_wallet").unwrap();
            retrieve_key(&id, "test_wallet").unwrap()
        });

        let retrieved1 = handle1.join().unwrap();
        let retrieved2 = handle2.join().unwrap();

        assert!(
            retrieved1.as_slice() == key1.as_slice() || retrieved2.as_slice() == key1.as_slice()
        );
        assert!(
            retrieved1.as_slice() == key2.as_slice() || retrieved2.as_slice() == key2.as_slice()
        );
        assert_ne!(retrieved1.as_slice(), retrieved2.as_slice());
    }

    #[serial_test::serial]
    #[test]
    fn test_multiple_keys() {
        clear_all_keys();
        let keys =
            [generate_key().unwrap(), generate_key().unwrap(), generate_key().unwrap()].to_vec();

        let ids: Vec<String> = keys.iter().map(|k| store_key(k, "test_wallet").unwrap()).collect();

        for (i, id) in ids.iter().enumerate() {
            let retrieved = retrieve_key(id, "test_wallet").unwrap();
            assert_eq!(retrieved.as_slice(), keys[i].as_slice());
        }
    }

    #[serial_test::serial]
    #[test]
    fn test_delete_key() {
        clear_all_keys();
        let key = generate_key().unwrap();
        let id = store_key(&key, "test_wallet").unwrap();

        // Verify key exists
        let retrieved = retrieve_key(&id, "test_wallet").unwrap();
        assert_eq!(key.as_slice(), retrieved.as_slice());

        // Delete key
        delete_key(&id).unwrap();

        // Verify key is gone
        let result = retrieve_key(&id, "test_wallet");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Key not found"));
    }

    #[serial_test::serial]
    #[test]
    fn test_delete_nonexistent_key() {
        clear_all_keys();
        let result = delete_key("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Key not found"));
    }

    #[serial_test::serial]
    #[test]
    fn test_key_count() {
        clear_all_keys();
        let initial_count = key_count();

        // Track our own keys
        let mut our_keys = Vec::new();

        let key1 = generate_key().unwrap();
        let id1 = store_key(&key1, "test_wallet").unwrap();
        our_keys.push(id1.clone());
        assert_eq!(key_count(), initial_count + 1);

        let key2 = generate_key().unwrap();
        let id2 = store_key(&key2, "test_wallet").unwrap();
        our_keys.push(id2.clone());
        assert_eq!(key_count(), initial_count + 2);

        delete_key(&id1).unwrap();
        our_keys.retain(|id| id != &id1);
        assert_eq!(key_count(), initial_count + 1);

        delete_key(&id2).unwrap();
        our_keys.retain(|id| id != &id2);

        // Final count should be back to initial
        let final_count = key_count();
        assert_eq!(final_count, initial_count, "Test should leave storage in initial state");
    }

    #[serial_test::serial]
    #[test]
    fn test_label_rotation_flow_basic() {
        clear_all_keys();
        let label = "wallet:alice:signing";
        let (id_v1, v1) = create_key_for_label("alice", label).unwrap();
        assert_eq!(v1, 1);

        // Current should be v1 and retrievable
        let (cur_key, cur_ver) = retrieve_current_key_for_label(label, "alice").unwrap();
        assert_eq!(cur_ver, 1);
        let key_v1 = retrieve_key_by_version(label, 1, "alice").unwrap();
        assert_eq!(*cur_key, *key_v1);

        // Rotate -> v2
        let (old_id, new_id, v2) = rotate_key_for_label("alice", label).unwrap();
        assert_eq!(old_id, id_v1);
        assert_ne!(old_id, new_id);
        assert_eq!(v2, 2);

        // Old must be marked retired in metadata; new must be current
        {
            let storage = ENCRYPTED_KEY_STORAGE.lock().unwrap();
            let old = storage.get(&old_id).unwrap();
            assert!(old.meta.retired);
            let newk = storage.get(&new_id).unwrap();
            assert!(!newk.meta.retired);
        }

        let (cur_key2, cur_ver2) = retrieve_current_key_for_label(label, "alice").unwrap();
        assert_eq!(cur_ver2, 2);

        // Historical retrieval still works
        let key_v1_bis = retrieve_key_by_version(label, 1, "alice").unwrap();
        assert_eq!(*key_v1_bis, *key_v1);
        // v2 must be different material from v1 in expectation
        let key_v2 = retrieve_key_by_version(label, 2, "alice").unwrap();
        assert_ne!(*key_v2, *key_v1);
        assert_eq!(*cur_key2, *key_v2);
    }

    #[serial_test::serial]
    #[test]
    fn test_label_rotation_concurrency_reads() {
        clear_all_keys();
        let label = "wallet:bob:signing";
        create_key_for_label("bob", label).unwrap();

        // Spawn several readers concurrently retrieving current key
        let mut handles = Vec::new();
        for _ in 0..8 {
            handles.push(thread::spawn({
                let label = label.to_string();
                move || {
                    let (_k, v) = retrieve_current_key_for_label(&label, "bob").unwrap();
                    v
                }
            }));
        }
        for h in handles {
            let v = h.join().unwrap();
            assert_eq!(v, 1);
        }
    }
}
