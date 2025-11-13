use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime /* NaiveDate */};
use sha2::{Digest, Sha256};
use sqlx::types::chrono::Utc;
use sqlx::{sqlite::SqlitePool, types::chrono::NaiveDateTime, FromRow, Row};
use std::any::Any;
use tracing::{debug, info, warn}; // for base64 engine decode

use crate::blockchain::bridge::{BridgeTransaction, BridgeTransactionStatus};
mod key_rotation;
pub use key_rotation::{KeyLabelRecord, KeyVersionRecord};

#[derive(Debug)]
pub struct WalletStorage {
    pool: SqlitePool,
    is_memory: bool,
}

impl WalletStorage {
    pub async fn new() -> Result<Self> {
        // default path (will create directories if needed)
        Self::new_with_url("sqlite://./data/wallet.db?mode=rwc").await
    }

    pub async fn new_with_url(database_url: &str) -> Result<Self> {
        // normalize sqlite URLs: accept "sqlite:" or "sqlite://"
        let mut db_url = database_url.to_string();
        if db_url.starts_with("sqlite:") && !db_url.starts_with("sqlite://") {
            db_url = db_url.replacen("sqlite:", "sqlite://", 1);
        }

        // ensure parent directory exists for file-backed sqlite URLs
        if let Some(path) = db_url.strip_prefix("sqlite://") {
            let (path_only, query) = path
                .split_once('?')
                .map(|(p, q)| (p.to_string(), Some(q)))
                .unwrap_or_else(|| (path.to_string(), None));

            // On Windows, urls like sqlite:///C:/path will produce a leading '/'
            // Normalize by removing leading '/' before drive letter.
            #[cfg(windows)]
            let path_only = {
                if path_only.starts_with('/') && path_only.len() > 2 {
                    let bytes = path_only.as_bytes();
                    if bytes[2] == b':' {
                        path_only[1..].to_string()
                    } else {
                        path_only
                    }
                } else {
                    path_only
                }
            };

            if path_only != ":memory:" && !path_only.is_empty() {
                if let Some(parent) = std::path::Path::new(&path_only).parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            warn!("Failed to create database dir {:?}: {}", parent, e);
                        }
                    }
                }

                // Rebuild db_url to normalized form; preserve query params
                let is_windows_abs = cfg!(windows)
                    && path_only.len() > 1
                    && path_only.as_bytes().get(1) == Some(&b':');
                let prefix = if is_windows_abs { "sqlite:///" } else { "sqlite://" };

                if let Some(query_str) = query {
                    db_url = format!("{}{}?{}", prefix, path_only, query_str);
                } else {
                    db_url = format!("{}{}", prefix, path_only);
                }
            }
        }

        // connect using normalized db_url
        // Avoid logging full DB URL (may contain secrets). Log scheme and path length for diagnostics.
        let safe_db_url_info = if let Some((scheme, rest)) = db_url.split_once("://") {
            format!("{}://(redacted, len={})", scheme, rest.len())
        } else {
            "(invalid db_url format)".to_string()
        };
        tracing::info!(db = %safe_db_url_info, "[storage] connecting to database");
        let is_memory = db_url.contains(":memory:");

        // 配置数据库连接池（企业级配置）
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;
        use std::time::Duration;
        
        let connect_options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| anyhow::anyhow!("Invalid database URL: {}", e))?
            .create_if_missing(true)  // 数据库不存在时自动创建
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)  // WAL模式，更好的并发性能
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);  // 平衡性能和安全性

        let pool = SqlitePoolOptions::new()
            .max_connections(20)  // 最大连接数
            .min_connections(2)   // 最小连接数
            .acquire_timeout(Duration::from_secs(30))  // 获取连接超时30秒
            .idle_timeout(Duration::from_secs(600))    // 空闲连接10分钟后关闭
            .max_lifetime(Duration::from_secs(1800))   // 连接最大生命周期30分钟
            .connect_with(connect_options)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

        let storage = Self { pool, is_memory };
        storage.initialize_schema().await?;

        info!("Wallet storage initialized");
        Ok(storage)
    }

    pub fn is_in_memory(&self) -> bool {
        self.is_memory
    }

    async fn initialize_schema(&self) -> Result<()> {
        debug!("Initializing database schema");

        // Wallets table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS wallets (
                id TEXT PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                encrypted_data BLOB NOT NULL,
                quantum_safe BOOLEAN NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create wallets table: {}", e))?;

        // Transactions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transactions (
                id TEXT PRIMARY KEY,
                wallet_id TEXT NOT NULL,
                tx_hash TEXT NOT NULL,
                network TEXT NOT NULL,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                amount TEXT NOT NULL,
                fee TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                confirmed_at DATETIME,
                integrity_hash TEXT NOT NULL,
                FOREIGN KEY (wallet_id) REFERENCES wallets (id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create transactions table: {}", e))?;

        // Audit logs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_id TEXT,
                action TEXT NOT NULL,
                details TEXT,
                ip_address TEXT,
                user_agent TEXT,
                created_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create audit_logs table: {}", e))?;

        // Audit logs HMAC integrity table (separate to avoid changing existing schema columns)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs_hmac (
                audit_id INTEGER PRIMARY KEY,
                mac TEXT NOT NULL,
                FOREIGN KEY(audit_id) REFERENCES audit_logs(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create audit_logs_hmac table: {}", e))?;

        // Bridge Transactions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bridge_transactions (
                id TEXT PRIMARY KEY,
                from_wallet TEXT NOT NULL,
                from_chain TEXT NOT NULL,
                to_chain TEXT NOT NULL,
                token TEXT NOT NULL,
                amount TEXT NOT NULL,
                status TEXT NOT NULL,
                source_tx_hash TEXT,
                destination_tx_hash TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                fee_amount TEXT,
                estimated_completion_time DATETIME
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_wallets_name ON wallets (name)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions (wallet_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_transactions_tx_hash ON transactions (tx_hash)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_audit_logs_wallet_id ON audit_logs (wallet_id)",
        )
        .execute(&self.pool)
        .await?;

        debug!("Database schema initialized");
        // Nonces table: persistent nonce reservations per network/address to avoid
        // replay across multi-instance deployments. next_nonce stores the next
        // available nonce (i.e. the value to return when reserving), so reserving
        // increments it atomically within a transaction.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS nonces (
                network TEXT NOT NULL,
                address TEXT NOT NULL,
                next_nonce INTEGER NOT NULL,
                updated_at DATETIME NOT NULL,
                PRIMARY KEY (network, address)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create nonces table: {}", e))?;
        // Initialize key rotation schema (non-fatal if fails? No—bubble up)
        key_rotation::init_schema(&self.pool).await?;
        Ok(())
    }

    pub async fn store_wallet(
        &self,
        name: &str,
        encrypted_data: &[u8],
        quantum_safe: bool,
    ) -> Result<()> {
        debug!("Storing wallet: {}", name);

        let wallet_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();

        sqlx::query(
            r#"
            INSERT INTO wallets (id, name, encrypted_data, quantum_safe, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&wallet_id)
        .bind(name)
        .bind(encrypted_data)
        .bind(quantum_safe)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store wallet: {}", e))?;

        // Log the action
        self.log_action(
            &wallet_id,
            "wallet_created",
            &format!("Wallet '{}' created", name),
            None,
            None,
        )
        .await?;

        debug!("Stored wallet: {}", name);
        Ok(())
    }

    pub async fn load_wallet(&self, name: &str) -> Result<(Vec<u8>, bool)> {
        debug!("Loading wallet: {}", name);

        let row =
            sqlx::query("SELECT id, encrypted_data, quantum_safe FROM wallets WHERE name = ?1")
                .bind(name)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to load wallet: {}", e))?;

        match row {
            Some(row) => {
                let wallet_id: String = row.get("id");
                let encrypted_data: Vec<u8> = row.get("encrypted_data");
                let quantum_safe: bool = row.get("quantum_safe");

                // Log the action
                self.log_action(
                    &wallet_id,
                    "wallet_accessed",
                    &format!("Wallet '{}' accessed", name),
                    None,
                    None,
                )
                .await?;

                debug!("Wallet loaded: {}", name);
                Ok((encrypted_data, quantum_safe))
            }
            None => Err(anyhow::anyhow!("Wallet not found: {}", name)),
        }
    }

    pub async fn list_wallets(&self) -> Result<Vec<WalletMetadata>> {
        debug!("Listing all wallets");

        let wallets = sqlx::query_as::<_, WalletMetadata>(
                "SELECT id, name, quantum_safe, created_at, updated_at FROM wallets ORDER BY created_at DESC"
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list wallets: {}", e))?;

        debug!("Listed {} wallets", wallets.len());
        Ok(wallets)
    }

    pub async fn update_wallet_encrypted_data(
        &self,
        name: &str,
        encrypted_data: &[u8],
    ) -> Result<()> {
        debug!("Updating wallet encrypted_data: {}", name);

        let now = Utc::now().naive_utc();
        let result = sqlx::query(
            r#"
            UPDATE wallets SET encrypted_data = ?1, updated_at = ?2 WHERE name = ?3
            "#,
        )
        .bind(encrypted_data)
        .bind(now)
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update wallet: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Wallet not found: {}", name));
        }

        Ok(())
    }

    /// 删除钱包
    ///
    /// # Errors
    /// 返回错误如果数据库操作失败
    pub async fn delete_wallet(&self, name: &str) -> Result<()> {
        debug!("Deleting wallet: {}", name);

        // Get wallet ID first
        let row = sqlx::query("SELECT id FROM wallets WHERE name = ?1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to find wallet: {}", e))?;

        let wallet_id = match row {
            Some(row) => row.get::<String, _>("id"),
            None => {
                return Err(anyhow::anyhow!("Wallet not found: {}", name));
            }
        };

        // Delete wallet
        let result = sqlx::query("DELETE FROM wallets WHERE name = ?1")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete wallet: {}", e))?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Wallet not found: {}", name));
        }

        // Log the action
        self.log_action(
            &wallet_id,
            "wallet_deleted",
            &format!("Wallet '{}' deleted", name),
            None,
            None,
        )
        .await?;

        warn!("Wallet deleted: {}", name);
        Ok(())
    }

    pub async fn store_transaction(&self, tx_data: &TransactionRecord) -> Result<()> {
        debug!("Storing transaction: {}", tx_data.tx_hash);

        // Calculate integrity hash
        let integrity_hash = Self::calculate_transaction_integrity_hash(tx_data);

        sqlx::query(
                r#"
            INSERT INTO transactions (id, wallet_id, tx_hash, network, from_address, to_address, amount, fee, status, created_at, confirmed_at, integrity_hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#
            )
            .bind(&tx_data.id)
            .bind(&tx_data.wallet_id)
            .bind(&tx_data.tx_hash)
            .bind(&tx_data.network)
            .bind(&tx_data.from_address)
            .bind(&tx_data.to_address)
            .bind(&tx_data.amount)
            .bind(&tx_data.fee)
            .bind(&tx_data.status)
            .bind(tx_data.created_at)
            .bind(tx_data.confirmed_at)
            .bind(integrity_hash)
            .execute(&self.pool).await
            .map_err(|e| anyhow::anyhow!("Failed to store transaction: {}", e))?;

        debug!("Transaction stored: {}", tx_data.tx_hash);
        Ok(())
    }

    pub async fn get_wallet_transactions(&self, wallet_id: &str) -> Result<Vec<TransactionRecord>> {
        debug!("Getting transactions for wallet: {}", wallet_id);

        let transactions = sqlx::query_as::<_, TransactionRecord>(
                r#"
            SELECT id, wallet_id, tx_hash, network, from_address, to_address, amount, fee, status, created_at, confirmed_at, integrity_hash
            FROM transactions 
            WHERE wallet_id = ?1 
            ORDER BY created_at DESC
            "#
            ).bind(wallet_id)
            .fetch_all(&self.pool).await
            .map_err(|e| anyhow::anyhow!("Failed to get transactions: {}", e))?;

        // Verify integrity of each transaction
        for tx in &transactions {
            Self::verify_transaction_integrity(tx)?;
        }

        debug!("Retrieved {} transactions", transactions.len());
        Ok(transactions)
    }

    pub async fn log_action(
        &self,
        wallet_id: &str,
        action: &str,
        details: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<()> {
        // Insert audit row and capture inserted row id from this statement result
        let res = sqlx::query(
            r#"
            INSERT INTO audit_logs (wallet_id, action, details, ip_address, user_agent, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(wallet_id)
        .bind(action)
        .bind(details)
        .bind(ip_address)
        .bind(user_agent)
        .bind(Utc::now().naive_utc())
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to log action: {}", e))?;
        let audit_id: i64 = res.last_insert_rowid();

        // Compute HMAC over the log content for integrity; key comes from WALLET_ENC_KEY as stable KEK (or test-env).
        let mac = Self::compute_audit_mac(
            audit_id as i64,
            wallet_id,
            action,
            details,
            ip_address,
            user_agent,
        )?;

        sqlx::query(
            r#"
            INSERT INTO audit_logs_hmac (audit_id, mac) VALUES (?1, ?2)
            "#,
        )
        .bind(audit_id)
        .bind(mac)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store audit mac: {}", e))?;

        Ok(())
    }

    pub async fn get_audit_logs(&self, wallet_id: Option<&str>) -> Result<Vec<AuditLog>> {
        let (query, params): (&str, Vec<&str>) = match wallet_id {
            Some(id) => {
                ("SELECT * FROM audit_logs WHERE wallet_id = ?1 ORDER BY created_at DESC", vec![id])
            }
            None => ("SELECT * FROM audit_logs ORDER BY created_at DESC", vec![]),
        };

        let mut query_builder = sqlx::query(query);
        for param in params {
            query_builder = query_builder.bind(param);
        }

        let logs = query_builder
            .try_map(|row: sqlx::sqlite::SqliteRow| AuditLog::from_row(&row))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get audit logs: {}", e))?;

        // Verify HMAC integrity for each audit log row
        for log in &logs {
            if let Err(e) = self.verify_audit_log_mac(log).await {
                return Err(anyhow::anyhow!("Audit log integrity failed for id {}: {}", log.id, e));
            }
        }

        Ok(logs)
    }

    /// Calculate integrity hash for transaction data to prevent tampering
    fn calculate_transaction_integrity_hash(tx: &TransactionRecord) -> String {
        let mut hasher = Sha256::new();
        hasher.update(tx.id.as_bytes());
        hasher.update(tx.wallet_id.as_bytes());
        hasher.update(tx.tx_hash.as_bytes());
        hasher.update(tx.network.as_bytes());
        hasher.update(tx.from_address.as_bytes());
        hasher.update(tx.to_address.as_bytes());
        hasher.update(tx.amount.as_bytes());
        hasher.update(tx.fee.as_bytes());
        hasher.update(tx.status.as_bytes());
        hasher.update(tx.created_at.timestamp().to_le_bytes());
        if let Some(confirmed_at) = tx.confirmed_at {
            hasher.update(confirmed_at.timestamp().to_le_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Verify transaction integrity by checking the stored hash against calculated hash
    fn verify_transaction_integrity(tx: &TransactionRecord) -> Result<()> {
        let calculated_hash = Self::calculate_transaction_integrity_hash(tx);
        if calculated_hash != tx.integrity_hash {
            return Err(anyhow::anyhow!(
                "Transaction integrity check failed for tx {}: expected {}, got {}",
                tx.id,
                tx.integrity_hash,
                calculated_hash
            ));
        }
        Ok(())
    }

    // --- Audit log MAC helpers ---
    fn load_audit_hmac_key() -> Result<[u8; 32]> {
        use zeroize::Zeroize;
        let b64 = std::env::var("WALLET_ENC_KEY")
            .map_err(|_| anyhow::anyhow!("WALLET_ENC_KEY not set for audit MAC"))?;
        let mut raw = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|_| anyhow::anyhow!("WALLET_ENC_KEY must be base64(32)"))?;
        if raw.len() != 32 {
            raw.zeroize();
            return Err(anyhow::anyhow!("WALLET_ENC_KEY must be 32 bytes"));
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

    fn compute_audit_mac(
        id: i64,
        wallet_id: &str,
        action: &str,
        details: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let key = Self::load_audit_hmac_key()?;
        let mut mac = HmacSha256::new_from_slice(&key)
            .map_err(|e| anyhow::anyhow!("HMAC init failed: {}", e))?;
        mac.update(&id.to_le_bytes());
        mac.update(wallet_id.as_bytes());
        mac.update(action.as_bytes());
        mac.update(details.as_bytes());
        if let Some(ip) = ip_address {
            mac.update(ip.as_bytes());
        }
        if let Some(ua) = user_agent {
            mac.update(ua.as_bytes());
        }
        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    async fn verify_audit_log_mac(&self, log: &AuditLog) -> Result<()> {
        let row = sqlx::query("SELECT mac FROM audit_logs_hmac WHERE audit_id = ?1")
            .bind(log.id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load audit mac: {}", e))?;

        let Some(row) = row else {
            return Err(anyhow::anyhow!("Missing audit mac"));
        };
        let stored_mac: String = row.get::<String, _>("mac");
        let calc = Self::compute_audit_mac(
            log.id,
            log.wallet_id.as_deref().unwrap_or(""),
            &log.action,
            log.details.as_deref().unwrap_or(""),
            log.ip_address.as_deref(),
            log.user_agent.as_deref(),
        )?;
        if stored_mac != calc {
            return Err(anyhow::anyhow!("MAC mismatch"));
        }
        Ok(())
    }
}

// Key rotation persistence API
impl WalletStorage {
    pub async fn rotation_upsert_label(
        &self,
        label: &str,
        current_version: i64,
        current_id: Option<&str>,
    ) -> Result<()> {
        key_rotation::upsert_label(&self.pool, label, current_version, current_id).await
    }

    pub async fn rotation_insert_version(
        &self,
        label: &str,
        version: i64,
        key_id: &str,
    ) -> Result<()> {
        key_rotation::insert_version(&self.pool, label, version, key_id).await
    }

    pub async fn rotation_mark_retired(&self, label: &str, version: i64) -> Result<()> {
        key_rotation::mark_retired(&self.pool, label, version).await
    }

    pub async fn rotation_inc_usage(&self, label: &str, version: i64) -> Result<()> {
        key_rotation::inc_usage(&self.pool, label, version).await
    }

    pub async fn rotation_get_label(
        &self,
        label: &str,
    ) -> Result<Option<key_rotation::KeyLabelRecord>> {
        key_rotation::get_label(&self.pool, label).await
    }

    pub async fn rotation_get_version(
        &self,
        label: &str,
        version: i64,
    ) -> Result<Option<key_rotation::KeyVersionRecord>> {
        key_rotation::get_version(&self.pool, label, version).await
    }
}

// Bridge Transaction Storage
impl WalletStorage {
    pub async fn store_bridge_transaction(&self, tx: &BridgeTransaction) -> Result<()> {
        let status_str = serde_json::to_string(&tx.status)?;
        sqlx::query(
            r#"
            INSERT INTO bridge_transactions (id, from_wallet, from_chain, to_chain, token, amount, status, source_tx_hash, destination_tx_hash, created_at, updated_at, fee_amount, estimated_completion_time)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
        )
        .bind(&tx.id)
        .bind(&tx.from_wallet)
        .bind(&tx.from_chain)
        .bind(&tx.to_chain)
        .bind(&tx.token)
        .bind(&tx.amount)
        .bind(status_str)
        .bind(&tx.source_tx_hash)
        .bind(&tx.destination_tx_hash)
        .bind(tx.created_at)
        .bind(tx.updated_at)
        .bind(&tx.fee_amount)
        .bind(tx.estimated_completion_time)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_bridge_transaction(&self, id: &str) -> Result<BridgeTransaction> {
        let row = sqlx::query("SELECT * FROM bridge_transactions WHERE id = ?1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

        let status_str: String = row.get("status");
        let status: BridgeTransactionStatus = serde_json::from_str(&status_str)?;

        let tx = BridgeTransaction {
            id: row.get("id"),
            from_wallet: row.get("from_wallet"),
            from_chain: row.get("from_chain"),
            to_chain: row.get("to_chain"),
            token: row.get("token"),
            amount: row.get("amount"),
            status,
            source_tx_hash: row.get("source_tx_hash"),
            destination_tx_hash: row.get("destination_tx_hash"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            fee_amount: row.get("fee_amount"),
            estimated_completion_time: row.get("estimated_completion_time"),
        };
        Ok(tx)
    }

    pub async fn update_bridge_transaction_status(
        &self,
        id: &str,
        status: BridgeTransactionStatus,
        source_tx_hash: Option<String>,
    ) -> Result<()> {
        let status_str = serde_json::to_string(&status)?;
        let now = Utc::now();
        sqlx::query("UPDATE bridge_transactions SET status = ?1, updated_at = ?2, source_tx_hash = COALESCE(?3, source_tx_hash) WHERE id = ?4")
            .bind(status_str)
            .bind(now)
            .bind(source_tx_hash)
            .bind(id)
            .execute(&self.pool).await?;
        Ok(())
    }

    /// List bridge transactions with optional filtering
    pub async fn list_bridge_transactions(
        &self,
        wallet_name: Option<&str>,
        from_chain: Option<&str>,
        to_chain: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<BridgeTransaction>, usize)> {
        // Build count query with filters
        let mut count_query = "SELECT COUNT(*) FROM bridge_transactions WHERE 1=1".to_string();
        let mut conditions = Vec::new();
        let mut params: Vec<&str> = Vec::new();
        
        if let Some(wallet) = wallet_name {
            conditions.push("from_wallet = ?");
            params.push(wallet);
        }
        if let Some(from) = from_chain {
            conditions.push("from_chain = ?");
            params.push(from);
        }
        if let Some(to) = to_chain {
            conditions.push("to_chain = ?");
            params.push(to);
        }
        
        if !conditions.is_empty() {
            count_query.push_str(" AND ");
            count_query.push_str(&conditions.join(" AND "));
        }
        
        // Count total
        let mut count_stmt = sqlx::query_scalar::<_, i64>(&count_query);
        for param in &params {
            count_stmt = count_stmt.bind(param);
        }
        let total_row = count_stmt.fetch_one(&self.pool).await?;
        let total = total_row as usize;
        
        // Build select query
        let mut select_query = "SELECT * FROM bridge_transactions WHERE 1=1".to_string();
        if !conditions.is_empty() {
            select_query.push_str(" AND ");
            select_query.push_str(&conditions.join(" AND "));
        }
        select_query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");
        
        // Fetch paginated results
        let mut select_stmt = sqlx::query(&select_query);
        for param in &params {
            select_stmt = select_stmt.bind(param);
        }
        let rows = select_stmt
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;
        
        let mut transactions = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: BridgeTransactionStatus = serde_json::from_str(&status_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse bridge status: {}", e))?;
            
            transactions.push(BridgeTransaction {
                id: row.get("id"),
                from_wallet: row.get("from_wallet"),
                from_chain: row.get("from_chain"),
                to_chain: row.get("to_chain"),
                token: row.get("token"),
                amount: row.get("amount"),
                status,
                source_tx_hash: row.get("source_tx_hash"),
                destination_tx_hash: row.get("destination_tx_hash"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                fee_amount: row.get("fee_amount"),
                estimated_completion_time: row.get("estimated_completion_time"),
            });
        }
        
        Ok((transactions, total))
    }
}

impl Clone for WalletStorage {
    fn clone(&self) -> Self {
        // Clone the underlying pool
        Self { pool: self.pool.clone(), is_memory: self.is_memory }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct WalletMetadata {
    pub id: String,
    pub name: String,
    pub quantum_safe: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub struct TransactionRecord {
    pub id: String,
    pub wallet_id: String,
    pub tx_hash: String,
    pub network: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub fee: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub integrity_hash: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub wallet_id: Option<String>,
    pub action: String,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait WalletStorageTrait {
    fn as_any(&self) -> &dyn Any;
    async fn store_wallet(&self, name: &str, data: &[u8], quantum_safe: bool) -> Result<()>;
    async fn load_wallet(&self, name: &str) -> Result<(Vec<u8>, bool)>;
    async fn list_wallets(&self) -> Result<Vec<WalletMetadata>>;
    async fn delete_wallet(&self, name: &str) -> Result<()>;
    async fn update_wallet_encrypted_data(&self, name: &str, data: &[u8]) -> Result<()>;
    async fn store_bridge_transaction(&self, tx: &BridgeTransaction) -> Result<()>;
    async fn get_bridge_transaction(&self, id: &str) -> Result<BridgeTransaction>;
    async fn update_bridge_transaction_status(
        &self,
        id: &str,
        status: BridgeTransactionStatus,
        source_tx_hash: Option<String>,
    ) -> Result<()>;

    // Key rotation persistence API on the trait so callers using dyn object can access them
    async fn rotation_upsert_label(
        &self,
        label: &str,
        current_version: i64,
        current_id: Option<&str>,
    ) -> Result<()>;
    async fn rotation_insert_version(&self, label: &str, version: i64, key_id: &str) -> Result<()>;
    async fn rotation_mark_retired(&self, label: &str, version: i64) -> Result<()>;
    async fn rotation_inc_usage(&self, label: &str, version: i64) -> Result<()>;
    async fn rotation_get_label(&self, label: &str) -> Result<Option<KeyLabelRecord>>;
    async fn rotation_get_version(
        &self,
        label: &str,
        version: i64,
    ) -> Result<Option<KeyVersionRecord>>;

    // Persistent nonce API to avoid replay across multiple instances.
    // reserve_next_nonce should return the current nonce and atomically
    // increment the stored next_nonce value. If no row exists, callers may
    // provide `initial` to seed the value.
    async fn reserve_next_nonce(&self, network: &str, address: &str, initial: u64) -> Result<u64>;
    // mark a nonce as used; this will set next_nonce to max(next_nonce, nonce+1)
    async fn mark_nonce_used(&self, network: &str, address: &str, nonce: u64) -> Result<()>;
}

// Implement the trait for WalletStorage by delegating to methods above
#[async_trait]
impl WalletStorageTrait for WalletStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }
    async fn store_wallet(&self, name: &str, data: &[u8], quantum_safe: bool) -> Result<()> {
        self.store_wallet(name, data, quantum_safe).await
    }

    async fn load_wallet(&self, name: &str) -> Result<(Vec<u8>, bool)> {
        self.load_wallet(name).await
    }

    async fn list_wallets(&self) -> Result<Vec<WalletMetadata>> {
        self.list_wallets().await
    }

    async fn delete_wallet(&self, name: &str) -> Result<()> {
        self.delete_wallet(name).await
    }

    async fn update_wallet_encrypted_data(&self, name: &str, data: &[u8]) -> Result<()> {
        self.update_wallet_encrypted_data(name, data).await
    }

    async fn store_bridge_transaction(&self, tx: &BridgeTransaction) -> Result<()> {
        self.store_bridge_transaction(tx).await
    }

    async fn get_bridge_transaction(&self, id: &str) -> Result<BridgeTransaction> {
        self.get_bridge_transaction(id).await
    }

    async fn update_bridge_transaction_status(
        &self,
        id: &str,
        status: BridgeTransactionStatus,
        source_tx_hash: Option<String>,
    ) -> Result<()> {
        self.update_bridge_transaction_status(id, status, source_tx_hash).await
    }

    async fn rotation_upsert_label(
        &self,
        label: &str,
        current_version: i64,
        current_id: Option<&str>,
    ) -> Result<()> {
        self.rotation_upsert_label(label, current_version, current_id).await
    }
    async fn rotation_insert_version(&self, label: &str, version: i64, key_id: &str) -> Result<()> {
        self.rotation_insert_version(label, version, key_id).await
    }
    async fn rotation_mark_retired(&self, label: &str, version: i64) -> Result<()> {
        self.rotation_mark_retired(label, version).await
    }
    async fn rotation_inc_usage(&self, label: &str, version: i64) -> Result<()> {
        self.rotation_inc_usage(label, version).await
    }
    async fn rotation_get_label(&self, label: &str) -> Result<Option<KeyLabelRecord>> {
        self.rotation_get_label(label).await
    }
    async fn rotation_get_version(
        &self,
        label: &str,
        version: i64,
    ) -> Result<Option<KeyVersionRecord>> {
        self.rotation_get_version(label, version).await
    }

    async fn reserve_next_nonce(&self, network: &str, address: &str, initial: u64) -> Result<u64> {
        // Use SQLite UPSERT to atomically increment next_nonce when row exists,
        // otherwise insert a seeded next_nonce = initial+1. After the upsert,
        // read the stored next_nonce and return next_nonce - 1 as the reserved
        // nonce.

        // Perform upsert: if row exists, increment next_nonce; else insert initial+1
        let now = Utc::now().naive_utc();
        let seed = (initial as i64) + 1;
        // SQLite UPSERT syntax
        let upsert_sql = r#"
            INSERT INTO nonces (network, address, next_nonce, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(network, address) DO UPDATE SET next_nonce = next_nonce + 1, updated_at = excluded.updated_at
        "#;
        sqlx::query(upsert_sql)
            .bind(network)
            .bind(address)
            .bind(seed)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("upsert nonce failed: {}", e))?;

        // Read back the stored next_nonce
        let row = sqlx::query("SELECT next_nonce FROM nonces WHERE network = ?1 AND address = ?2")
            .bind(network)
            .bind(address)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("select nonce failed: {}", e))?;
        let next_nonce: i64 = row.get("next_nonce");
        // reserved is next_nonce - 1
        Ok((next_nonce - 1) as u64)
    }

    async fn mark_nonce_used(&self, network: &str, address: &str, nonce: u64) -> Result<()> {
        // Ensure stored next_nonce >= nonce + 1. Use UPSERT to either insert
        // or update to desired value when higher.
        let desired = (nonce as i64) + 1;
        // Try to update only when desired > next_nonce using a conditional update
        let updated = sqlx::query("UPDATE nonces SET next_nonce = ?1, updated_at = ?2 WHERE network = ?3 AND address = ?4 AND next_nonce < ?1")
            .bind(desired)
            .bind(Utc::now().naive_utc())
            .bind(network)
            .bind(address)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("update nonce failed: {}", e))?;

        if updated.rows_affected() == 0 {
            // Either row didn't exist or condition didn't hold; ensure row exists with at least desired
            // Use INSERT OR REPLACE pattern to set desired if absent.
            sqlx::query("INSERT INTO nonces (network, address, next_nonce, updated_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(network,address) DO UPDATE SET next_nonce = MAX(next_nonce, excluded.next_nonce), updated_at = excluded.updated_at")
                .bind(network)
                .bind(address)
                .bind(desired)
                .bind(Utc::now().naive_utc())
                .execute(&self.pool)
                .await
                .map_err(|e| anyhow::anyhow!("insert/replace nonce failed: {}", e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_wallet_storage_operations() {
        // 设置测试环境变量 (32 bytes key)
        std::env::set_var("WALLET_ENC_KEY", "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI="); // base64 encoded 32-byte test key
        
        // Use in-memory sqlite for tests
        let storage = WalletStorage::new_with_url("sqlite::memory:").await.unwrap();

        // Test store wallet
        let wallet_data = b"test wallet data";
        storage.store_wallet("test-wallet", wallet_data, false).await.unwrap();

        // Test load wallet
        let (loaded_data, quantum_safe) = storage.load_wallet("test-wallet").await.unwrap();
        assert_eq!(loaded_data, wallet_data);
        assert!(!quantum_safe);

        // Test list wallets
        let wallets = storage.list_wallets().await.unwrap();
        assert!(!wallets.is_empty());
        assert!(wallets.iter().any(|w| w.name == "test-wallet"));

        // Test delete wallet
        storage.delete_wallet("test-wallet").await.unwrap();

        // Verify deletion
        let result = storage.load_wallet("test-wallet").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transaction_integrity_verification() {
        // 设置测试环境变量 (32 bytes key)
        std::env::set_var("WALLET_ENC_KEY", "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI="); // base64 encoded 32-byte test key
        
        let storage = WalletStorage::new_with_url("sqlite::memory:").await.unwrap();

        // First create a wallet to satisfy foreign key constraint
        let wallet_data = b"test wallet data";
        storage.store_wallet("test-wallet", wallet_data, false).await.unwrap();

        // Get the wallet ID (UUID) that was generated
        let wallets = storage.list_wallets().await.unwrap();
        let wallet_id = wallets.iter().find(|w| w.name == "test-wallet").unwrap().id.clone();

        let tx = TransactionRecord {
            id: "test-tx-integrity".to_string(),
            wallet_id: wallet_id.clone(),
            tx_hash: "0x1234567890abcdef".to_string(),
            network: "eth".to_string(),
            from_address: "0x1234567890123456789012345678901234567890".to_string(),
            to_address: "0x0987654321098765432109876543210987654321".to_string(),
            amount: "1.0".to_string(),
            fee: "0.01".to_string(),
            status: "pending".to_string(),
            created_at: Utc::now(),
            confirmed_at: None,
            integrity_hash: String::new(), // Will be calculated during storage
        };

        // Store transaction (integrity hash will be calculated and stored)
        storage.store_transaction(&tx).await.unwrap();

        // Retrieve transaction (integrity should be verified)
        let transactions = storage.get_wallet_transactions(&wallet_id).await.unwrap();
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].id, "test-tx-integrity");

        // Verify integrity hash is not empty
        assert!(!transactions[0].integrity_hash.is_empty());

        // Test tampering detection: manually modify the database to simulate tampering
        // We'll use raw SQL to update the amount field directly, bypassing integrity calculation
        sqlx::query("UPDATE transactions SET amount = ? WHERE id = ?")
            .bind("999.0") // Tampered amount
            .bind("test-tx-integrity")
            .execute(&storage.pool)
            .await
            .unwrap();

        // Now retrieving should fail due to integrity check
        let result = storage.get_wallet_transactions(&wallet_id).await;
        assert!(result.is_err(), "Expected integrity check to fail for tampered transaction");
        assert!(result.unwrap_err().to_string().contains("integrity check failed"));
    }

    #[tokio::test]
    async fn test_bridge_transaction_storage() {
        let storage = WalletStorage::new_with_url("sqlite::memory:").await.unwrap();

        let tx = BridgeTransaction {
            id: "test-tx-123".to_string(),
            from_wallet: "wallet1".to_string(),
            from_chain: "eth".to_string(),
            to_chain: "polygon".to_string(),
            token: "USDC".to_string(),
            amount: "100.0".to_string(),
            status: BridgeTransactionStatus::Initiated,
            source_tx_hash: None,
            destination_tx_hash: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            fee_amount: Some("1.0".to_string()),
            estimated_completion_time: Some(Utc::now() + chrono::Duration::hours(1)),
        };

        // Store transaction
        storage.store_bridge_transaction(&tx).await.unwrap();

        // Retrieve transaction
        let retrieved = storage.get_bridge_transaction("test-tx-123").await.unwrap();
        assert_eq!(retrieved.id, tx.id);
        assert_eq!(retrieved.status, BridgeTransactionStatus::Initiated);

        // Update status
        storage
            .update_bridge_transaction_status(
                "test-tx-123",
                BridgeTransactionStatus::Completed,
                Some("0x123".to_string()),
            )
            .await
            .unwrap();

        let updated = storage.get_bridge_transaction("test-tx-123").await.unwrap();
        assert_eq!(updated.status, BridgeTransactionStatus::Completed);
        assert_eq!(updated.source_tx_hash, Some("0x123".to_string()));
    }

    #[tokio::test]
    async fn test_key_rotation_persistence() {
        let storage = WalletStorage::new_with_url("sqlite::memory:").await.unwrap();

        // Insert initial label and version
        storage.rotation_upsert_label("wallet:alice:signing", 1, None).await.unwrap();
        storage.rotation_insert_version("wallet:alice:signing", 1, "key-uuid-v1").await.unwrap();

        let lbl = storage.rotation_get_label("wallet:alice:signing").await.unwrap().unwrap();
        assert_eq!(lbl.current_version, 1);
        assert_eq!(lbl.current_id, None);

        // Promote current id and add a new version
        storage
            .rotation_upsert_label("wallet:alice:signing", 1, Some("key-uuid-v1"))
            .await
            .unwrap();
        let lbl = storage.rotation_get_label("wallet:alice:signing").await.unwrap().unwrap();
        assert_eq!(lbl.current_id.as_deref(), Some("key-uuid-v1"));

        // Rotate -> version 2
        storage.rotation_insert_version("wallet:alice:signing", 2, "key-uuid-v2").await.unwrap();
        storage
            .rotation_upsert_label("wallet:alice:signing", 2, Some("key-uuid-v2"))
            .await
            .unwrap();
        storage.rotation_mark_retired("wallet:alice:signing", 1).await.unwrap();
        storage.rotation_inc_usage("wallet:alice:signing", 2).await.unwrap();

        let v1 = storage.rotation_get_version("wallet:alice:signing", 1).await.unwrap().unwrap();
        assert!(v1.retired);
        let v2 = storage.rotation_get_version("wallet:alice:signing", 2).await.unwrap().unwrap();
        assert!(!v2.retired);
        assert_eq!(v2.usage_count, 1);

        let lbl = storage.rotation_get_label("wallet:alice:signing").await.unwrap().unwrap();
        assert_eq!(lbl.current_version, 2);
        assert_eq!(lbl.current_id.as_deref(), Some("key-uuid-v2"));
    }
}
