use crate::security::SecretVec;
use anyhow;
use lazy_static::lazy_static;
use secp256k1::{Message, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use zeroize::Zeroizing;
use zeroize::{Zeroize, ZeroizeOnDrop};

// 使用 lazy_static 初始化全局可变事务状态存储
lazy_static! {
    static ref TX_STATUS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

// Monotonic counter to build unique transaction hashes for tests and examples
static TX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 安全的私钥包装器，确保内存被安全擦除
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecurePrivateKey {
    #[zeroize(skip)]
    pub algorithm: String,
    // Store key bytes in a Zeroizing container so clones/returns are explicit and
    // the buffer is zeroed when dropped.
    pub key_data: Zeroizing<Vec<u8>>,
}

impl SecurePrivateKey {
    pub fn new(key_data: Vec<u8>) -> Self {
        Self { algorithm: "secp256k1".to_string(), key_data: Zeroizing::new(key_data) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.key_data
    }
}

/// 返回对全局状态存储的引用
fn status_store() -> &'static Mutex<HashMap<String, String>> {
    &TX_STATUS
}

#[derive(Clone, Debug)]
pub struct Wallet {
    pub address: String,
    // Store private key in a zeroizing wrapper to avoid lingering secrets in memory
    private_key: Zeroizing<Vec<u8>>, // raw bytes
    mnemonic: Zeroizing<String>,     // keep mnemonic zeroized as well
}

impl Wallet {
    pub fn from_mnemonic(mnemonic: &str) -> Result<Self, anyhow::Error> {
        // Simple implementation for testing - in production this would derive from actual mnemonic
        // Store mnemonic and private key in zeroizing containers to avoid lingering plaintext in memory.
        let pk_bytes = format!("priv_key_from_{}", mnemonic).into_bytes();
        Ok(Wallet {
            address: format!("0x{}", hex::encode([0u8; 20])),
            private_key: Zeroizing::new(pk_bytes),
            mnemonic: Zeroizing::new(mnemonic.to_string()),
        })
    }

    /// Return a zeroizing copy of the private key bytes. Callers receive a
    /// `Zeroizing<Vec<u8>>` which will be zeroed on drop. Prefer passing
    /// `&[u8]` to low-level signing APIs instead of materializing copies.
    pub fn private_key_bytes(&self) -> Zeroizing<Vec<u8>> {
        // Clone the Zeroizing<Vec<u8>> so the returned value is also zeroizing.
        self.private_key.clone()
    }

    /// Consume the wallet and return the underlying private key bytes as a
    /// `Zeroizing<Vec<u8>>`. This transfers ownership without creating an
    /// intermediate plain Vec<u8>, ensuring the buffer will be zeroized when dropped.
    pub fn take_private_key(self) -> Zeroizing<Vec<u8>> {
        // Move the Zeroizing<Vec<u8>> out of the struct. No extra cloning.
        self.private_key
    }

    /// Return the mnemonic as a zeroizing string (clone) so callers receive an owned
    /// `Zeroizing<String>` which will be zeroed on drop. Prefer passing references
    /// to signing flows instead of cloning when possible.
    pub fn mnemonic_secret(&self) -> Zeroizing<String> {
        self.mnemonic.clone()
    }

    /// Deprecated: original mnemonic() returned `&str` and exposed a live reference to
    /// secret material. Prefer `mnemonic_secret()` which returns an owned zeroizing
    /// buffer that will be cleared on drop.
    #[deprecated(note = "Use mnemonic_secret() which returns an owned Zeroizing<String>")]
    pub fn mnemonic(&self) -> &str {
        &self.mnemonic
    }

    /// Implement a redacted Debug representation so secrets are not accidentally printed.
    pub fn fmt_redacted(&self) -> String {
        format!(
            "Wallet {{ address: {}, private_key: <redacted>, mnemonic: <redacted> }}",
            self.address
        )
    }
}

pub fn create_wallet(name: &str) -> Result<Wallet, String> {
    if name.is_empty() || name.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Invalid wallet name".to_string());
    }
    Ok(Wallet {
        address: format!("0x{}", "0".repeat(40)),
        private_key: Zeroizing::new(format!("priv_key_{}", name).into_bytes()),
        mnemonic: Zeroizing::new(format!("{}ball", "test ".repeat(11))),
    })
}

pub fn bridge_assets_amount(amount: Option<&str>) -> Result<f64, String> {
    match amount {
        Some(s) => match s.parse::<f64>() {
            Ok(v) if v > 0.0 => Ok(v),
            _ => Err("Invalid amount".to_string()),
        },
        None => Err("Invalid amount".to_string()),
    }
}

pub fn generate_log(msg: &str) -> String {
    // 简单日志格式化（实际代码应使用 tracing/log）
    format!("LOG: {}", msg)
}

pub fn query_balance(_account: &str) -> u128 {
    0
}

#[derive(Clone, Debug)]
pub struct Transaction {
    pub id: String,
    pub to: String,
    pub amount: u64,
    // Replay protection fields
    pub nonce: u64,     // Transaction nonce to prevent replay
    pub chain_id: u64,  // Chain ID to prevent cross-chain replay
    pub gas_limit: u64, // Gas limit for the transaction
    pub gas_price: u64, // Gas price in wei
    pub data: Vec<u8>,  // Transaction data/payload
}

#[derive(Clone, Debug)]
pub struct TransactionParams {
    pub to: String,
    pub amount: u64,
}

impl TransactionParams {
    pub fn new(to: &str, amount: u64) -> Self {
        Self { to: to.into(), amount }
    }
}

pub fn construct_transaction(params: TransactionParams) -> Transaction {
    Transaction {
        id: "tx_constructed".into(),
        to: params.to,
        amount: params.amount,
        nonce: 0,               // Default nonce
        chain_id: 1,            // Ethereum mainnet
        gas_limit: 21000,       // Standard ETH transfer gas limit
        gas_price: 20000000000, // 20 gwei
        data: vec![],           // No data for simple transfers
    }
}

pub fn create_transaction() -> Transaction {
    Transaction {
        id: "tx_local_1".into(),
        to: "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into(),
        amount: 42,
        nonce: 0,
        chain_id: 1,
        gas_limit: 21000,
        gas_price: 20000000000,
        data: vec![],
    }
}

pub fn generate_private_key() -> Zeroizing<Vec<u8>> {
    Zeroizing::new("priv_test_key".to_string().into_bytes())
}

pub fn derive_public_key_from_bytes(private_key_bytes: &[u8]) -> String {
    // Validate private key length
    if private_key_bytes.len() != 32 {
        panic!("Private key must be exactly 32 bytes");
    }

    // Generate the actual secp256k1 public key
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(private_key_bytes).expect("Invalid private key");
    let keypair = secp256k1::KeyPair::from_secret_key(&secp, &secret_key);

    // Return the public key as hex string (compressed format)
    hex::encode(keypair.public_key().serialize())
}

pub fn sign_transaction(tx: &Transaction, private_key_bytes: &[u8]) -> Result<SecretVec, String> {
    // 使用安全的私钥包装器
    let secure_key = SecurePrivateKey::new(private_key_bytes.to_vec());

    // Validate private key length
    if secure_key.as_bytes().len() != 32 {
        return Err("Private key must be exactly 32 bytes".to_string());
    }

    // Create secp256k1 context and sign
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(secure_key.as_bytes())
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let keypair = secp256k1::KeyPair::from_secret_key(&secp, &secret_key);

    // Hash the complete transaction data for signing (improved integrity)
    let mut tx_hasher = Sha256::new();
    tx_hasher.update(b"WALLET_TX_V1"); // Version prefix to prevent cross-protocol attacks
    tx_hasher.update(tx.id.as_bytes());
    tx_hasher.update(tx.to.as_bytes());
    tx_hasher.update(tx.amount.to_le_bytes());
    tx_hasher.update(tx.nonce.to_le_bytes());
    tx_hasher.update(tx.chain_id.to_le_bytes());
    tx_hasher.update(tx.gas_limit.to_le_bytes());
    tx_hasher.update(tx.gas_price.to_le_bytes());
    tx_hasher.update(&tx.data);
    let message_hash = tx_hasher.finalize();
    let message = Message::from_slice(&message_hash)
        .map_err(|e| format!("Failed to create message hash: {}", e))?;

    // Create a recoverable signature so we can extract the recovery id (v)
    let rec_sig = secp.sign_ecdsa_recoverable(&message, &keypair.secret_key());

    // Serialize to compact (r||s) and recovery id
    let (rec_id, compact) = rec_sig.serialize_compact();

    // Normalize s to low-S using centralized helper to avoid signature malleability
    let compact_norm = {
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&compact);
        crate::crypto::signature_utils::ensure_low_s(&arr)
    };

    // Now recompute the recovery id so it matches the normalized (r||s) compact bytes.
    // Try recid 0 and 1 and pick the one that recovers the public key derived from the keypair.
    let pubkey = keypair.public_key();

    let mut chosen_v: Option<u8> = None;
    for cand in 0..=1u8 {
        if let Ok(rec_id_try) = secp256k1::ecdsa::RecoveryId::from_i32(cand as i32) {
            if let Ok(rec_sig_try) =
                secp256k1::ecdsa::RecoverableSignature::from_compact(&compact_norm, rec_id_try)
            {
                if let Ok(recovered_pk) = secp.recover_ecdsa(&message, &rec_sig_try) {
                    // Compare recovered public key with the keypair's public key
                    if recovered_pk == pubkey {
                        chosen_v = Some(cand);
                        break;
                    }
                }
            }
        }
    }

    let v = match chosen_v {
        Some(x) => x,
        None => {
            // As a fallback, use the original rec_id if it is 0/1; otherwise fail.
            let fallback = rec_id.to_i32() as u8;
            if fallback <= 1 {
                fallback
            } else {
                return Err("Failed to determine recovery id after normalization".to_string());
            }
        }
    };

    // Return r||s||v (65 bytes) to make the recovery id explicit for callers.
    let mut out = Vec::with_capacity(65);
    out.extend_from_slice(&compact_norm);
    out.push(v);

    // secure_key 在函数结束时会自动通过ZeroizeOnDrop擦除内存
    // Wrap the output in a SecretVec so callers receive a zeroizing buffer.
    Ok(crate::security::secret::vec_to_secret(out))
}

pub fn verify_signature(tx: &Transaction, sig: &[u8], public_key: &str) -> bool {
    // Validate input parameters
    if sig.is_empty() {
        tracing::warn!("Signature verification failed: empty signature");
        return false;
    }

    if public_key.is_empty() {
        tracing::warn!("Signature verification failed: empty public key");
        return false;
    }

    // Hash the complete transaction data (must match sign_transaction)
    let mut hasher = Sha256::new();
    hasher.update(b"WALLET_TX_V1"); // Version prefix to prevent cross-protocol attacks
    hasher.update(tx.id.as_bytes());
    hasher.update(tx.to.as_bytes());
    hasher.update(tx.amount.to_le_bytes());
    hasher.update(tx.nonce.to_le_bytes());
    hasher.update(tx.chain_id.to_le_bytes());
    hasher.update(tx.gas_limit.to_le_bytes());
    hasher.update(tx.gas_price.to_le_bytes());
    hasher.update(&tx.data);
    let message_hash = hasher.finalize();

    let message = match Message::from_slice(&message_hash) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::error!("Signature verification failed: invalid message hash: {}", e);
            return false;
        }
    };

    // Accept either 64-byte compact (r||s) or 65-byte r||s||v signatures
    let compact_sig: &[u8] = match sig.len() {
        64 => sig,
        65 => {
            // validate v (last byte)
            let v = sig[64];
            if v > 1 {
                tracing::warn!("Signature verification failed: invalid recovery id v={}", v);
                return false;
            }
            &sig[..64]
        }
        _ => {
            tracing::warn!(
                "Signature verification failed: unexpected signature length: {}",
                sig.len()
            );
            return false;
        }
    };

    // Parse the signature
    let signature = match secp256k1::ecdsa::Signature::from_compact(compact_sig) {
        Ok(sig) => sig,
        Err(e) => {
            tracing::warn!("Signature verification failed: invalid signature format: {}", e);
            return false;
        }
    };

    // Parse the public key from hex string
    let pub_key_bytes = match hex::decode(public_key) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("Signature verification failed: invalid public key hex: {}", e);
            return false;
        }
    };

    let secp = Secp256k1::new();
    let public_key_obj = match secp256k1::PublicKey::from_slice(&pub_key_bytes) {
        Ok(pk) => pk,
        Err(e) => {
            tracing::warn!("Signature verification failed: invalid public key: {}", e);
            return false;
        }
    };

    match secp.verify_ecdsa(&message, &signature, &public_key_obj) {
        Ok(_) => {
            tracing::debug!("Signature verification successful for transaction {}", tx.id);
            true
        }
        Err(e) => {
            tracing::warn!(
                "Signature verification failed: invalid signature for transaction {}: {}",
                tx.id,
                e
            );
            false
        }
    }
}

pub fn is_signature_valid(sig: &[u8], public_key: &str) -> bool {
    // For MVP, we'll do a basic validation of signature format
    // In production, this should verify against actual transaction data
    // Accept 64-byte compact or 65-byte compact+v. If v is present, it must be 0 or 1.
    if public_key.is_empty() {
        return false;
    }
    match sig.len() {
        64 => true,
        65 => sig[64] <= 1,
        _ => false,
    }
}

pub fn send_transaction(wallet: &str, amount: Option<u64>) -> Result<String, String> {
    if amount.unwrap_or(0) == 0 {
        return Err("Invalid amount".to_string());
    }
    if wallet.is_empty() || wallet.chars().any(|c| !c.is_alphanumeric() && c != '_') {
        return Err("Invalid wallet name".to_string());
    }

    // Include a monotonically increasing suffix to avoid collisions across parallel tests
    let ctr = TX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let hash = format!("0xhash_{}_{}", wallet, ctr);
    let mut map = status_store().lock().unwrap();
    map.insert(hash.clone(), "sent".into());
    Ok(hash)
}

pub fn confirm_transaction(id_or_hash: String) -> Result<bool, String> {
    let mut map = status_store().lock().unwrap();
    map.insert(id_or_hash, "confirmed".into());
    Ok(true)
}

pub fn get_transaction_status(id_or_hash: String) -> String {
    let map = status_store().lock().unwrap();
    map.get(&id_or_hash).cloned().unwrap_or_else(|| "pending".into())
}

pub fn calculate_bridge_fee(amount: Option<&str>) -> Result<f64, String> {
    match amount {
        Some(s) => match s.parse::<f64>() {
            Ok(v) if v > 0.0 => Ok(v * 0.01),
            _ => Err("Invalid amount".to_string()),
        },
        None => Err("Invalid amount".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tx_status_set_get_clear() {
        let tx = "tx123";
        let hash = send_transaction(tx, Some(1)).expect("send tx");
        assert_eq!(get_transaction_status(hash.clone()), "sent".to_string());
        assert!(confirm_transaction(hash.clone()).unwrap());
        assert_eq!(get_transaction_status(hash), "confirmed".to_string());
    }

    #[test]
    fn create_wallet_validation() {
        assert!(create_wallet("").is_err());
        assert!(create_wallet("validName1").is_ok());
    }

    #[test]
    fn test_sign_and_verify_transaction() {
        let tx = create_transaction();
        // Use a proper 32-byte private key for testing
        let private_key_bytes = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78,
            0x9a, 0xbc, 0xde, 0xf0,
        ];
        let public_key = derive_public_key_from_bytes(&private_key_bytes);

        // Sign the transaction (should return r||s||v)
        let signature =
            sign_transaction(&tx, &private_key_bytes).expect("Failed to sign transaction");

        // Signature should be 65 bytes (r||s||v)
        assert_eq!(signature.len(), 65);

        // Verify the signature (signature is SecretVec - borrow as slice)
        assert!(verify_signature(&tx, signature.as_ref(), &public_key));
        assert!(is_signature_valid(signature.as_ref(), &public_key));

        // Test with wrong transaction
        let wrong_tx = Transaction {
            id: "wrong_tx".to_string(),
            to: tx.to.clone(),
            amount: tx.amount,
            nonce: tx.nonce,
            chain_id: tx.chain_id,
            gas_limit: tx.gas_limit,
            gas_price: tx.gas_price,
            data: tx.data.clone(),
        };
        assert!(!verify_signature(&wrong_tx, &signature, &public_key));

        // Test with invalid signature
        let invalid_sig = vec![0xFF, 0xFF, 0xFF];
        assert!(!verify_signature(&tx, &invalid_sig, &public_key));
        assert!(!is_signature_valid(&invalid_sig, &public_key));

        // Test with bad v (length 65 but v=2)
        let mut bad_v = <zeroize::Zeroizing<Vec<u8>> as AsRef<[u8]>>::as_ref(&signature).to_vec();
        bad_v[64] = 2u8; // invalid recovery id
        assert!(!verify_signature(&tx, &bad_v, &public_key));
        assert!(!is_signature_valid(&bad_v, &public_key));
    }
}
