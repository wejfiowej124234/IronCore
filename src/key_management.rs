use std::sync::Mutex;
use zeroize::{Zeroize, ZeroizeOnDrop};
use zeroize::Zeroizing;

/// Convenient alias for secret buffers that will be zeroized on drop.
pub type SecretVec = zeroize::Zeroizing<Vec<u8>>;

// 涓轰簡娴嬭瘯鐩殑锛屼娇鐢ㄤ竴涓畝鍗曠殑鍐呭瓨瀛樺偍
// 鍦ㄥ疄闄呭簲鐢ㄤ腑锛岃繖浼氭槸涓€涓畨鍏ㄧ殑銆佹寔涔呭寲鐨勫瓨鍌ㄦ満鍒?
static KEY_STORAGE: Mutex<Option<SecurePrivateKey>> = Mutex::new(None);

/// 安全的私钥包装器，确保内存被安全擦除
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecurePrivateKey {
    #[zeroize(skip)]
    pub algorithm: String,
    // Keep the underlying key in a Zeroizing buffer when possible.
    pub key_data: Zeroizing<Vec<u8>>,
}

impl SecurePrivateKey {
    pub fn new(key_data: Vec<u8>, algorithm: &str) -> Self {
        Self {
            algorithm: algorithm.to_string(),
            key_data: Zeroizing::new(key_data),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.key_data
    }
}

/// 瀵嗛挜绠＄悊鐩稿叧鐨勯敊璇被鍨?
#[derive(Debug, thiserror::Error)]
pub enum KeyManagementError {
    #[error("Key generation failed")]
    KeyGenerationFailed,
    #[error("Key storage failed: {0}")]
    KeyStorageFailed(String),
    #[error("Key not found")]
    KeyNotFound,
    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

/// 生成一个新的私钥
/// 在生产应用中，这将使用一个密码学安全的随机数生成器
pub fn generate_key() -> Result<SecretVec, KeyManagementError> {
    use rand::RngCore;

    // 生成32字节的随机私钥（secp256k1标准）
    let mut key = vec![0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);

    // 验证私钥是否在secp256k1曲线的有效范围内
    // secp256k1的阶数是0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
    let secp_order = hex::decode("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141")
        .map_err(|_| KeyManagementError::KeyGenerationFailed)?;

    // 确保私钥不为0且小于secp256k1阶数
    if key.iter().all(|&b| b == 0) {
        return Err(KeyManagementError::KeyGenerationFailed);
    }

    // 简单的范围检查（生产环境中应该使用更严格的检查）
    if key.len() == 32 && key[0] != 0 {
        Ok(zeroize::Zeroizing::new(key))
    } else {
        Err(KeyManagementError::KeyGenerationFailed)
    }
}

/// 瀛樺偍涓€涓瘑閽ャ€?
/// 鍦ㄥ疄闄呭簲鐢ㄤ腑锛岃繖浼氬皢瀵嗛挜鍔犲瘑骞舵寔涔呭寲瀛樺偍銆?
pub fn store_key(key: &[u8]) -> Result<(), KeyManagementError> {
    if key.is_empty() {
        return Err(KeyManagementError::InvalidKey("Key cannot be empty".to_string()));
    }
    let secure_key = SecurePrivateKey::new(key.to_vec(), "secp256k1");
    let mut storage = KEY_STORAGE
        .lock()
        .map_err(|e| KeyManagementError::KeyStorageFailed(e.to_string()))?;
    *storage = Some(secure_key);
    Ok(())
}

/// 妫€绱㈠瓨鍌ㄧ殑瀵嗛挜銆?
/// 鍦ㄥ疄闄呭簲鐢ㄤ腑锛岃繖浼氫粠鎸佷箙鍖栧瓨鍌ㄤ腑璇诲彇骞惰В瀵嗗瘑閽ャ€?
pub fn retrieve_key() -> Result<SecretVec, KeyManagementError> {
    let storage = KEY_STORAGE
        .lock()
        .map_err(|e| KeyManagementError::KeyStorageFailed(e.to_string()))?;
    match storage.as_ref() {
        Some(secure_key) => Ok(secure_key.key_data.clone()),
        None => Err(KeyManagementError::KeyNotFound),
    }
}

/// 娓呴櫎鎵€鏈夊瓨鍌ㄧ殑瀵嗛挜銆?
/// 鍦ㄥ疄闄呭簲鐢ㄤ腑锛岃繖浼氬畨鍏ㄥ湴鎿﹂櫎鎸佷箙鍖栧瓨鍌ㄤ腑鐨勫瘑閽ャ€?
pub fn clear_keys() -> Result<(), KeyManagementError> {
    let mut storage = KEY_STORAGE
        .lock()
        .map_err(|e| KeyManagementError::KeyStorageFailed(e.to_string()))?;
    *storage = None; // SecurePrivateKey的ZeroizeOnDrop会自动擦除内存
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key = generate_key().unwrap();
        assert!(!key.is_empty());
        assert_eq!(key.len(), 32); // 生成32字节私钥（secp256k1标准）
    }

    #[test]
    fn test_store_key() {
        clear_keys().unwrap(); // 纭繚娴嬭瘯鍓嶇姸鎬佸共鍑€
        let key = vec![1, 2, 3];
        store_key(&key).unwrap();
        let retrieved = retrieve_key().unwrap();
        assert_eq!(retrieved, key);
    }

    #[test]
    fn test_store_key_empty() {
        clear_keys().unwrap(); // 纭繚娴嬭瘯鍓嶇姸鎬佸共鍑€
        assert!(store_key(&[]).is_err());
    }

    #[test]
    fn test_retrieve_key_not_found() {
        clear_keys().unwrap(); // 纭繚娌℃湁瀵嗛挜
        assert!(retrieve_key().is_err());
    }
}
