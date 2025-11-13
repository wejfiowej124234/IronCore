// Mock storage implementation when database feature is disabled
// This allows the project to compile without sqlx/rusqlite

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub quantum_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub network: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub wallet_id: Option<Uuid>,
    pub action: String,
    pub details: String,
    pub timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait WalletStorageTrait: Send + Sync {
    async fn save_wallet(
        &self,
        name: &str,
        encrypted_data: &[u8],
        quantum_safe: bool,
    ) -> Result<Uuid>;
    
    async fn load_wallet(&self, name: &str) -> Result<Vec<u8>>;
    
    async fn list_wallets(&self) -> Result<Vec<WalletMetadata>>;
    
    async fn delete_wallet(&self, name: &str) -> Result<()>;
    
    async fn save_transaction(&self, record: &TransactionRecord) -> Result<()>;
    
    async fn log_audit(&self, wallet_id: Option<Uuid>, action: &str, details: &str) -> Result<()>;
    
    async fn get_bridge_transaction(&self, _id: &Uuid) -> Result<()>;
    
    async fn rotation_get_version(&self, _label: &str, _version: u32) -> Result<()>;
    
    async fn rotation_get_label(&self, _label: &str) -> Result<Option<()>>;
    
    async fn update_wallet_encrypted_data(&self, name: &str, encrypted_data: &[u8]) -> Result<()>;
    
    async fn update_bridge_transaction_status(
        &self,
        _id: &str,
        _status: &str,
        _source_tx_hash: Option<String>,
    ) -> Result<()>;
    
    async fn rotation_insert_version(&self, _label: &str, _version: i64, _key_id: &str) -> Result<()>;
    
    async fn rotation_upsert_label(&self, _label: &str, _version: i64, _key_id: Option<&str>) -> Result<()>;
    
    async fn rotation_mark_retired(&self, _label: &str, _version: i64) -> Result<()>;
    
    async fn rotation_inc_usage(&self, _label: &str, _version: i64) -> Result<()>;
    
    async fn store_bridge_transaction(&self, _bt: &()) -> Result<()>;
    
    async fn mark_nonce_used(&self, _network: &str, _address: &str, _nonce: u64) -> Result<()>;
    
    async fn reserve_next_nonce(&self, _network: &str, _address: &str, _chain_nonce: u64) -> Result<u64>;
    
    fn is_in_memory(&self) -> bool;
}

#[derive(Clone)]
pub struct WalletStorage {
    wallets: Arc<Mutex<HashMap<String, WalletEntry>>>,
    transactions: Arc<Mutex<Vec<TransactionRecord>>>,
    audit_logs: Arc<Mutex<Vec<AuditLog>>>,
}

type WalletEntry = (Uuid, Vec<u8>, bool, DateTime<Utc>);

impl WalletStorage {
    pub async fn new(_db_path: &str) -> Result<Self> {
        Ok(Self {
            wallets: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            audit_logs: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn new_with_url(_url: &str) -> Result<Self> {
        Self::new("").await
    }
}

#[async_trait]
impl WalletStorageTrait for WalletStorage {
    async fn save_wallet(
        &self,
        name: &str,
        encrypted_data: &[u8],
        quantum_safe: bool,
    ) -> Result<Uuid> {
        let mut wallets = self.wallets.lock().await;
        let id = Uuid::new_v4();
        let now = Utc::now();
        wallets.insert(name.to_string(), (id, encrypted_data.to_vec(), quantum_safe, now));
        Ok(id)
    }

    async fn load_wallet(&self, name: &str) -> Result<Vec<u8>> {
        let wallets = self.wallets.lock().await;
        wallets
            .get(name)
            .map(|(_, data, _, _)| data.clone())
            .ok_or_else(|| anyhow!("Wallet not found: {}", name))
    }

    async fn list_wallets(&self) -> Result<Vec<WalletMetadata>> {
        let wallets = self.wallets.lock().await;
        Ok(wallets
            .iter()
            .map(|(name, (id, _, quantum_safe, created_at))| WalletMetadata {
                id: *id,
                name: name.clone(),
                created_at: *created_at,
                updated_at: *created_at,
                quantum_safe: *quantum_safe,
            })
            .collect())
    }

    async fn delete_wallet(&self, name: &str) -> Result<()> {
        let mut wallets = self.wallets.lock().await;
        wallets
            .remove(name)
            .ok_or_else(|| anyhow!("Wallet not found: {}", name))?;
        Ok(())
    }

    async fn save_transaction(&self, record: &TransactionRecord) -> Result<()> {
        let mut transactions = self.transactions.lock().await;
        transactions.push(record.clone());
        Ok(())
    }

    async fn log_audit(&self, wallet_id: Option<Uuid>, action: &str, details: &str) -> Result<()> {
        let mut audit_logs = self.audit_logs.lock().await;
        audit_logs.push(AuditLog {
            id: Uuid::new_v4(),
            wallet_id,
            action: action.to_string(),
            details: details.to_string(),
            timestamp: Utc::now(),
        });
        Ok(())
    }
    
    async fn get_bridge_transaction(&self, _id: &Uuid) -> Result<()> {
        Err(anyhow!("Bridge transactions not supported in mock storage"))
    }
    
    async fn rotation_get_version(&self, _label: &str, _version: u32) -> Result<()> {
        Err(anyhow!("Key rotation not supported in mock storage"))
    }
    
    async fn rotation_get_label(&self, _label: &str) -> Result<Option<()>> {
        Ok(None)
    }
    
    async fn update_wallet_encrypted_data(&self, name: &str, encrypted_data: &[u8]) -> Result<()> {
        let mut wallets = self.wallets.lock().await;
        if let Some(entry) = wallets.get(name).cloned() {
            let (id, _, quantum_safe, created_at) = entry;
            wallets.insert(name.to_string(), (id, encrypted_data.to_vec(), quantum_safe, created_at));
            Ok(())
        } else {
            Err(anyhow!("Wallet not found: {}", name))
        }
    }
    
    async fn update_bridge_transaction_status(
        &self,
        _id: &str,
        _status: &str,
        _source_tx_hash: Option<String>,
    ) -> Result<()> {
        Err(anyhow!("Bridge transactions not supported in mock storage"))
    }
    
    async fn rotation_insert_version(&self, _label: &str, _version: i64, _key_id: &str) -> Result<()> {
        Err(anyhow!("Key rotation not supported in mock storage"))
    }
    
    async fn rotation_upsert_label(&self, _label: &str, _version: i64, _key_id: Option<&str>) -> Result<()> {
        Err(anyhow!("Key rotation not supported in mock storage"))
    }
    
    async fn rotation_mark_retired(&self, _label: &str, _version: i64) -> Result<()> {
        Err(anyhow!("Key rotation not supported in mock storage"))
    }
    
    async fn rotation_inc_usage(&self, _label: &str, _version: i64) -> Result<()> {
        Err(anyhow!("Key rotation not supported in mock storage"))
    }
    
    async fn store_bridge_transaction(&self, _bt: &()) -> Result<()> {
        Err(anyhow!("Bridge transactions not supported in mock storage"))
    }
    
    async fn mark_nonce_used(&self, _network: &str, _address: &str, _nonce: u64) -> Result<()> {
        Err(anyhow!("Nonce tracking not supported in mock storage"))
    }
    
    async fn reserve_next_nonce(&self, _network: &str, _address: &str, chain_nonce: u64) -> Result<u64> {
        // Mock implementation: just return chain_nonce
        Ok(chain_nonce)
    }
    
    fn is_in_memory(&self) -> bool {
        true // Mock storage is always in-memory
    }
}
