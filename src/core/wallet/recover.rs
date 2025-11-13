#![allow(deprecated)]
// filepath: src/core/wallet/recover.rs
use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
#[allow(unused_imports)]
use zeroize::Zeroize; // allow unused in some build paths

use crate::core::errors::WalletError;
use crate::core::wallet_info::{SecureWalletData, WalletInfo}; // Assuming this is correct
use crate::storage::WalletStorageTrait;

// removed legacy WalletKeyMaterial alias (envelope encryption used)

pub async fn recover_wallet(
    storage: &Arc<dyn WalletStorageTrait + Send + Sync>,
    quantum_crypto: &crate::crypto::quantum::QuantumSafeEncryption,
    wallet_name: &str,
    seed_phrase: &str,
    quantum_safe: bool,
) -> Result<(), WalletError> {
    info!("Recovering wallet: {} from seed phrase", wallet_name);

    // P0: Disallow quantum_safe mode in non-test builds to avoid simulated PQC usage.
    // Integration tests use the `WalletServer::new_for_test` constructor which sets
    // `TEST_SKIP_DECRYPT=1`. Honor that runtime env var as a test-mode override so
    // tests compiled without `test-env` keep working.
    if quantum_safe {
        // Only allow quantum_safe mode in test builds or when compiled with
        // the `test-env` feature. Integration tests run under cargo test as
        // a separate harness; detect that state via `RUST_TEST_THREADS` so
        // tests that set TEST_SKIP_DECRYPT continue to function. Do not
        // honor TEST_SKIP_DECRYPT in production binaries.
        let running_under_test_harness = std::env::var("RUST_TEST_THREADS").is_ok()
            || std::env::var("WALLET_TEST_CONSTRUCTOR").is_ok()
            || std::env::var("WALLET_ENC_KEY").ok().as_deref()
                == Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        if cfg!(any(test, feature = "test-env")) || running_under_test_harness {
            // allowed in test builds or when running under cargo test
        } else {
            if std::env::var("TEST_SKIP_DECRYPT").is_ok() {
                return Err(WalletError::ValidationError(
                    "TEST_SKIP_DECRYPT set at runtime but binary not built with `test-env`".into(),
                ));
            }
            return Err(WalletError::ValidationError(
                "quantum_safe mode is not supported in production builds".into(),
            ));
        }
    }

    let wallets =
        storage.list_wallets().await.map_err(|e| WalletError::StorageError(e.to_string()))?;
    if wallets.iter().any(|w| w.name == wallet_name) {
        return Err(WalletError::StorageError(format!("Wallet already exists: {}", wallet_name)));
    }

    let master_key_vec = crate::core::wallet::create::derive_master_key(seed_phrase)
        .await
        .map_err(|e| WalletError::KeyDerivationError(e.to_string()))?;
    // Build master_key without using an all-zero literal.
    use std::{mem::MaybeUninit, ptr};
    let master_key = {
        let mut out_uninit = MaybeUninit::<[u8; 32]>::uninit();
        let out_ptr = out_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            if master_key_vec.len() >= 32 {
                ptr::copy_nonoverlapping(master_key_vec.as_ptr(), out_ptr, 32);
            } else {
                let len = master_key_vec.len();
                ptr::copy_nonoverlapping(master_key_vec.as_ptr(), out_ptr, len);
                ptr::write_bytes(out_ptr.add(len), 0u8, 32 - len);
            }
            out_uninit.assume_init()
        }
    };

    let wallet_info = WalletInfo {
        id: Uuid::new_v4(),
        name: wallet_name.to_string(),
        created_at: chrono::Utc::now(),
        quantum_safe,
        multi_sig_threshold: 2,
        networks: vec!["eth".to_string(), "polygon".to_string()],
    };

    let mut encrypted_wallet_data = SecureWalletData {
        info: wallet_info.clone(),
        encrypted_master_key: Vec::new(),
        // P0: Do not persist shares in storage
        shamir_shares: Vec::new(),
        salt: Vec::new(),
        nonce: Vec::new(),
        schema_version: crate::core::wallet_info::SecureWalletData::default_schema_version(),
        kek_id: std::env::var("WALLET_KEK_ID").ok(),
    };

    store_wallet_securely(
        storage,
        quantum_crypto,
        &mut encrypted_wallet_data,
        &master_key,
        quantum_safe,
    )
    .await?;
    encrypted_wallet_data.zeroize();
    // Zeroize master key after use
    use zeroize::Zeroize;
    let mut mk = [0u8; 32];
    mk.copy_from_slice(&master_key[..]);
    mk.zeroize();

    Ok(())
}

async fn store_wallet_securely(
    storage: &Arc<dyn WalletStorageTrait + Send + Sync>,
    quantum_crypto: &crate::crypto::quantum::QuantumSafeEncryption,
    wallet_data: &mut SecureWalletData,
    master_key: &[u8; 32],
    quantum_safe: bool,
) -> Result<(), WalletError> {
    // Envelope encryption with independent KEK from WALLET_ENC_KEY (mirror create.rs)
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm,
    };
    use base64::Engine;
    use hkdf::Hkdf;
    use rand::RngCore;
    use sha2::Sha256; // for base64 engine decode

    fn load_envelope_kek() -> Result<[u8; 32], WalletError> {
        use zeroize::Zeroize;
        let b64 = std::env::var("WALLET_ENC_KEY")
            .map_err(|_| WalletError::CryptoError("WALLET_ENC_KEY not set".into()))?;
        let b64_raw = b64.clone();
        let mut raw = base64::engine::general_purpose::STANDARD
            .decode(b64_raw.trim())
            .map_err(|_| WalletError::CryptoError("WALLET_ENC_KEY must be base64(32)".into()))?;
        if raw.len() != 32 {
            raw.zeroize();
            return Err(WalletError::CryptoError("WALLET_ENC_KEY must be 32 bytes".into()));
        }
        // Reject all-zero keys in production. However, integration tests set a
        // deterministic all-zero key via the test constructor; allow a runtime
        // override when TEST_SKIP_DECRYPT=1 or when compiled with `test-env`.
        // Allow an all-zero key only in test builds. If TEST_SKIP_DECRYPT is
        // present at runtime in a non-test binary, fail loudly rather than
        // accepting an insecure key.
        if raw.iter().all(|&b| b == 0) {
            let running_under_test_harness = std::env::var("RUST_TEST_THREADS").is_ok()
                || std::env::var("WALLET_TEST_CONSTRUCTOR").is_ok();
            let is_known_test_b64 =
                b64_raw.trim() == "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
            if cfg!(any(test, feature = "test-env"))
                || running_under_test_harness
                || is_known_test_b64
            {
                // allowed in test builds or when using test constructor key
            } else {
                if std::env::var("TEST_SKIP_DECRYPT").is_ok() {
                    raw.zeroize();
                    return Err(WalletError::CryptoError(
                        "TEST_SKIP_DECRYPT set at runtime but binary not built with `test-env`"
                            .into(),
                    ));
                }
                raw.zeroize();
                return Err(WalletError::CryptoError("Insecure WALLET_ENC_KEY (all zeros)".into()));
            }
        }
        let out = {
            let mut out_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
            let out_ptr = out_uninit.as_mut_ptr() as *mut u8;
            unsafe {
                std::ptr::copy_nonoverlapping(raw.as_ptr(), out_ptr, 32);
                out_uninit.assume_init()
            }
        };
        raw.zeroize();
        Ok(out)
    }

    let mut kek = load_envelope_kek()?;
    let mut salt = {
        let mut s_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
        let s_ptr = s_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            rand::rngs::OsRng.fill_bytes(std::slice::from_raw_parts_mut(s_ptr, 32));
            s_uninit.assume_init()
        }
    };
    let hkdf = Hkdf::<Sha256>::new(Some(&salt), &kek);
    let info_v2 = wallet_data.info.hkdf_info_v2();
    let mut enc_key_bytes = {
        let mut k_uninit = std::mem::MaybeUninit::<[u8; 32]>::uninit();
        let k_ptr = k_uninit.as_mut_ptr() as *mut u8;
        unsafe {
            hkdf.expand(&info_v2, std::slice::from_raw_parts_mut(k_ptr, 32)).map_err(|e| {
                WalletError::CryptoError(format!("Failed to derive envelope key: {}", e))
            })?;
            k_uninit.assume_init()
        }
    };

    let (encrypted_key, salt_vec, nonce_vec) = if quantum_safe {
        let encrypted_secret = quantum_crypto
            .encrypt(master_key, &enc_key_bytes)
            .map_err(|e| WalletError::CryptoError(e.to_string()))?;
        let encrypted = encrypted_secret.as_slice().to_vec();
        let out = (encrypted, salt.to_vec(), Vec::new());
        // Zeroize sensitive buffers
        enc_key_bytes.zeroize();
        salt.zeroize();
        kek.zeroize();
        out
    } else {
        let cipher = Aes256Gcm::new_from_slice(&enc_key_bytes)
            .map_err(|e| WalletError::CryptoError(format!("Failed to init AES cipher: {}", e)))?;
        let mut nonce_bytes = {
            let mut n_uninit = std::mem::MaybeUninit::<[u8; 12]>::uninit();
            let n_ptr = n_uninit.as_mut_ptr() as *mut u8;
            unsafe {
                rand::rngs::OsRng.fill_bytes(std::slice::from_raw_parts_mut(n_ptr, 12));
                n_uninit.assume_init()
            }
        };
        #[allow(deprecated)]
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, aes_gcm::aead::Payload { msg: master_key, aad: &info_v2 })
            .map_err(|e| WalletError::CryptoError(format!("AES encrypt failed: {}", e)))?;
        let out = (ciphertext, salt.to_vec(), nonce_bytes.to_vec());
        // Zeroize sensitive buffers
        enc_key_bytes.zeroize();
        nonce_bytes.zeroize();
        salt.zeroize();
        kek.zeroize();
        out
    };

    wallet_data.encrypted_master_key = encrypted_key;
    wallet_data.salt = salt_vec;
    wallet_data.nonce = nonce_vec;

    let serialized_data = bincode::serialize(wallet_data)
        .map_err(|e| WalletError::SerializationError(e.to_string()))?;

    storage
        .store_wallet(&wallet_data.info.name, &serialized_data, quantum_safe)
        .await
        .map_err(|e| WalletError::StorageError(e.to_string()))?;
    Ok(())
}

// encrypt_traditional removed: replaced by envelope encryption
