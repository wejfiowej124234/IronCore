// src/core/wallet_info.rs
use chrono::{DateTime, Utc};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalletInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub quantum_safe: bool,
    pub multi_sig_threshold: u8,
    pub networks: Vec<String>,
}

impl WalletInfo {
    /// Creates a new WalletInfo with default settings.
    pub fn new(name: &str, quantum_safe: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            created_at: Utc::now(),
            quantum_safe,
            multi_sig_threshold: 2,
            networks: vec!["eth".to_string(), "polygon".to_string()],
        }
    }

    /// Legacy AAD (v1): wallet name bytes
    pub fn aad_v1(&self) -> Vec<u8> {
        self.name.as_bytes().to_vec()
    }

    /// Stable AAD (v2): domain tag + wallet UUID bytes
    pub fn aad_v2(&self) -> Vec<u8> {
        let mut out = b"DEFISAFE-AAD-V2".to_vec();
        out.extend_from_slice(self.id.as_bytes());
        out
    }

    /// HKDF info (v1): label + v1 AAD
    pub fn hkdf_info_v1(&self) -> Vec<u8> {
        [b"wallet-master-key".as_ref(), self.aad_v1().as_slice()].concat()
    }

    /// HKDF info (v2): label + v2 AAD
    pub fn hkdf_info_v2(&self) -> Vec<u8> {
        [b"wallet-master-key-v2".as_ref(), self.aad_v2().as_slice()].concat()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecureWalletData {
    pub info: WalletInfo,
    pub encrypted_master_key: Vec<u8>,
    pub shamir_shares: Vec<Vec<u8>>,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    #[serde(default = "SecureWalletData::default_schema_version")]
    pub schema_version: u8,
    #[serde(default)]
    pub kek_id: Option<String>,
}

impl Zeroize for SecureWalletData {
    fn zeroize(&mut self) {
        self.encrypted_master_key.zeroize();
        self.shamir_shares.zeroize();
        self.salt.zeroize();
        self.nonce.zeroize();
        // Note: info does not contain sensitive data, so no need to zeroize
    }
}

impl ZeroizeOnDrop for SecureWalletData {}

impl SecureWalletData {
    /// Creates a new SecureWalletData with empty encrypted fields.
    pub fn new(info: WalletInfo) -> Self {
        Self {
            info,
            encrypted_master_key: Vec::new(),
            shamir_shares: Vec::new(),
            salt: Vec::new(),
            nonce: Vec::new(),
            schema_version: Self::default_schema_version(),
            kek_id: None,
        }
    }

    /// Zeroizes sensitive data manually.
    pub fn zeroize(&mut self) {
        <Self as Zeroize>::zeroize(self);
    }

    pub fn default_schema_version() -> u8 {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_info_new() {
        let info = WalletInfo::new("test_wallet", true);
        assert_eq!(info.name, "test_wallet");
        assert!(info.quantum_safe);
        assert_eq!(info.multi_sig_threshold, 2);
        assert!(info.networks.contains(&"eth".to_string()));
    }

    #[test]
    fn test_secure_wallet_data_new() {
        let info = WalletInfo::new("test_wallet", false);
        let secure_data = SecureWalletData::new(info.clone());
        assert_eq!(secure_data.info.name, "test_wallet");
        assert!(secure_data.encrypted_master_key.is_empty());
    }

    #[test]
    fn test_secure_wallet_data_zeroize() {
        let mut secure_data = SecureWalletData::new(WalletInfo::new("test", false));
        secure_data.encrypted_master_key = vec![1, 2, 3];
        secure_data.salt = vec![4, 5, 6];
        secure_data.nonce = vec![7, 8, 9];

        secure_data.zeroize();
        assert!(secure_data.encrypted_master_key.iter().all(|&x| x == 0));
        assert!(secure_data.salt.iter().all(|&x| x == 0));
        assert!(secure_data.nonce.iter().all(|&x| x == 0));
    }
}
