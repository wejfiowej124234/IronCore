use aes_gcm::KeyInit;
use aes_gcm::{aead::Aead, Aes256Gcm};
use rand::RngCore;
use zeroize::Zeroizing;

/// Encrypts a mnemonic using AES-256-GCM and returns the bytes to write to disk
/// Format: 12-byte nonce || ciphertext
pub fn encrypt_mnemonic_to_bytes(
    mnemonic: &str,
    key_bytes: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, String> {
    if key_bytes.len() != 32 {
        return Err("Encryption key must be 32 bytes".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|_| "Invalid key".to_string())?;

    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = {
        // Nonce::from_slice currently depends on older `generic-array` helpers in our
        // dependency tree which are marked deprecated. Allow the deprecated usage
        // here until dependencies can be upgraded across the workspace.
        #[allow(deprecated)]
        aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&nonce_bytes)
    };

    let seed_bytes_zero = Zeroizing::new(mnemonic.as_bytes().to_vec());

    let ciphertext = cipher
        .encrypt(nonce, aes_gcm::aead::Payload { msg: seed_bytes_zero.as_ref(), aad })
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts bytes produced by `encrypt_mnemonic_to_bytes` given the key and aad
pub fn decrypt_mnemonic_from_bytes(
    blob: &[u8],
    key_bytes: &[u8],
    aad: &[u8],
) -> Result<zeroize::Zeroizing<Vec<u8>>, String> {
    if key_bytes.len() != 32 {
        return Err("Encryption key must be 32 bytes".to_string());
    }
    if blob.len() < 12 {
        return Err("Blob too short".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|_| "Invalid key".to_string())?;

    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let nonce = {
        // See note above: allow deprecated constructor until transitive deps are updated.
        #[allow(deprecated)]
        aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(nonce_bytes)
    };

    let plaintext = cipher
        .decrypt(nonce, aes_gcm::aead::Payload { msg: ciphertext, aad })
        .map_err(|e| format!("Decryption failed: {}", e))?;

    // Return raw bytes wrapped in Zeroizing so callers receive secret bytes
    Ok(zeroize::Zeroizing::new(plaintext))
}
