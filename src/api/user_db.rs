//! User database operations module
//!
//! Separate user database (users.db), physically isolated from wallet database (wallets.db)
//!
//! Security features:
//! - Argon2id password hashing
//! - SQL injection protection (parameterized queries)
//! - Account lockout mechanism
//! - Token expiration management

use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions}};
use std::str::FromStr;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// User database connection pool
pub struct UserDatabase {
    pool: SqlitePool,
}

/// User model structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Wallet information (non-custodial mode: public info only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub name: String,
    pub address: Option<String>,  // Wallet address (public key)
    pub wallet_type: String,      // Wallet type: standard, multisig, etc.
    pub created_at: String,
}

/// Create user request payload
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub username: Option<String>,
}

/// Login request payload
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl UserDatabase {
    /// Get database connection pool reference (for use in other APIs)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// Create new user database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        tracing::info!("📂 Initializing user database: {}", database_url);
        
        // Print current working directory for debugging
        let current_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        tracing::info!("📁 Current working directory: {}", current_dir);

        // Use SqliteConnectOptions for explicit configuration
        let connect_options = SqliteConnectOptions::from_str(database_url)
            .context("Invalid database URL format")?
            .create_if_missing(true)  // Critical: Auto-create file if missing
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)  // Use WAL mode for better concurrency
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
        
        tracing::info!("📄 Connection options: create_if_missing=true, journal=WAL");

        // Create connection pool with options
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await
            .context("Failed to connect to users database")?;
        
        tracing::info!("✅ User database connected successfully");

        // Run migrations (create table structure)
        tracing::info!("📋 Running database migrations...");
        sqlx::query(include_str!("../../migrations/users/001_create_users_table.sql"))
            .execute(&pool)
            .await
            .context("Failed to run users database migrations")?;
        
        tracing::info!("✅ User database initialization complete");

        Ok(Self { pool })
    }

    /// Create initial demo user (if not exists)
    /// Reset demo user password on each startup to avoid hash mismatch
    pub async fn ensure_demo_user(&self) -> Result<()> {
        let demo_email = "demo@securewallet.local";
        let demo_id = "demo-user-0000-0000-000000000001";
        let demo_password = "Demo@123456";
        
        // Check if demo user already exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = ?)"
        )
        .bind(demo_email)
        .fetch_one(&self.pool)
        .await?;
        
        if exists {
            // User exists, update password hash to ensure it's always correct
            let password_hash = Self::hash_password(demo_password)?;
            sqlx::query(
                "UPDATE users SET password_hash = ?, failed_login_attempts = 0, locked_until = NULL WHERE email = ?"
            )
            .bind(&password_hash)
            .bind(demo_email)
            .execute(&self.pool)
            .await?;
            
            tracing::info!("✅ Demo user password reset: {}", demo_email);
        } else {
            // Create demo user
            let password_hash = Self::hash_password(demo_password)?;
            
            sqlx::query(
                "INSERT INTO users (id, email, username, password_hash) VALUES (?, ?, ?, ?)"
            )
            .bind(demo_id)
            .bind(demo_email)
            .bind("Demo User")
            .bind(password_hash)
            .execute(&self.pool)
            .await?;
            
            tracing::info!("✅ Demo user created: {}", demo_email);
        }
        
        Ok(())
    }
    
    /// Register new user
    pub async fn create_user(&self, req: CreateUserRequest) -> Result<User> {
        // Validate email format (enhanced)
        Self::validate_email(&req.email)?;
        
        // Validate password strength (maximum security - using validators.rs)
        Self::validate_password(&req.password)?;
        
        // Hash password using bcrypt
        let password_hash = Self::hash_password(&req.password)?;
        
        // Generate UUID for new user
        let user_id = Uuid::new_v4().to_string();
        
        // Insert user into database
        let result = sqlx::query(
            "INSERT INTO users (id, email, username, password_hash) VALUES (?, ?, ?, ?)"
        )
        .bind(&user_id)
        .bind(&req.email)
        .bind(&req.username)
        .bind(&password_hash)
        .execute(&self.pool)
        .await;
        
        // Catch UNIQUE constraint error and return friendly message
        let result = match result {
            Ok(r) => r,
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("UNIQUE constraint failed: users.email") {
                    anyhow::bail!("Email already registered, please use a different email")
                } else {
                    return Err(e.into());
                }
            }
        };
        
        // Verify insertion was successful
        if result.rows_affected() == 0 {
            anyhow::bail!("Failed to insert user (email may already exist)");
        }
        
        tracing::info!("✅ User registered successfully: {} ({})", req.email, user_id);
        
        // Query and return complete user information
        self.get_user_by_id(&user_id).await
    }
    
    /// User login verification
    pub async fn verify_login(&self, req: LoginRequest) -> Result<User> {
        tracing::info!("🔑 Starting login verification: email={}", req.email);
        
        // Query user from database
        let row = sqlx::query_as::<_, (String, String, i32, i32, Option<String>)>(
            "SELECT id, password_hash, is_active, failed_login_attempts, locked_until FROM users WHERE email = ?"
        )
        .bind(&req.email)
        .fetch_optional(&self.pool)
        .await?;
        
        if row.is_none() {
            tracing::error!("❌ User not found: email={}", req.email);
        }
        
        let row = row.ok_or_else(|| anyhow::anyhow!("Invalid email or password"))?;
        
        let user_id = row.0;
        let password_hash = row.1;
        let is_active = row.2 == 1;
        let failed_attempts = row.3;
        let locked_until = row.4;
        
        tracing::info!("   ✅ User found: id={}", user_id);
        tracing::info!("   Account active: {}", is_active);
        tracing::info!("   Failed attempts: {}", failed_attempts);
        tracing::info!("   Password hash: {}...", &password_hash[..password_hash.len().min(30)]);
        
        // Check account status
        if !is_active {
            anyhow::bail!("Account is disabled");
        }
        
        // Check if account is locked
        if let Some(locked) = locked_until {
            let locked_time = DateTime::parse_from_rfc3339(&locked)?;
            if locked_time > Utc::now() {
                anyhow::bail!("Account is locked. Try again later");
            }
        }
        
        // Verify password
        tracing::info!("🔐 Calling password verification...");
        let password_valid = Self::verify_password(&req.password, &password_hash)?;
        
        if !password_valid {
            tracing::error!("❌ Password verification failed!");
            // Increment failed login attempts
            let new_attempts = failed_attempts + 1;
            
            // Lock account for 30 minutes after 5 failed attempts
            if new_attempts >= 5 {
                let lock_until = Utc::now() + chrono::Duration::minutes(30);
                sqlx::query(
                    "UPDATE users SET failed_login_attempts = ?, locked_until = ? WHERE id = ?"
                )
                .bind(new_attempts)
                .bind(lock_until.to_rfc3339())
                .bind(&user_id)
                .execute(&self.pool)
                .await?;
                
                anyhow::bail!("Too many failed attempts. Account locked for 30 minutes");
            } else {
                sqlx::query("UPDATE users SET failed_login_attempts = ? WHERE id = ?")
                    .bind(new_attempts)
                    .bind(&user_id)
                    .execute(&self.pool)
                    .await?;
            }
            
            anyhow::bail!("Invalid email or password");
        }
        
        // Login successful: Reset failed attempts, update last login time
        sqlx::query(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, last_login_at = datetime('now', 'utc') WHERE id = ?"
        )
        .bind(&user_id)
        .execute(&self.pool)
        .await?;
        
        tracing::info!("✅ User login successful: {} ({})", req.email, user_id);
        
        // Return user information
        self.get_user_by_id(&user_id).await
    }
    
    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, i32)>(
            "SELECT id, email, username, created_at, last_login_at, is_active FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        
        // Parse time string (SQLite storage format: "2025-11-04 15:07:21")
        let created_at = Self::parse_sqlite_datetime(&row.3)?;
        let last_login_at = row.4.and_then(|s| Self::parse_sqlite_datetime(&s).ok());
        
        Ok(User {
            id: row.0,
            email: row.1,
            username: row.2,
            created_at,
            last_login_at,
            is_active: row.5 == 1,
        })
    }
    
    /// Parse SQLite datetime format
    fn parse_sqlite_datetime(s: &str) -> Result<DateTime<Utc>> {
        // SQLite datetime('now', 'utc') format: "2025-11-04 15:07:21"
        // Convert to ISO 8601: "2025-11-04T15:07:21Z"
        let iso_format = format!("{}Z", s.replace(" ", "T"));
        DateTime::parse_from_rfc3339(&iso_format)
            .map(|dt| dt.with_timezone(&Utc))
            .context(format!("Failed to parse datetime: {}", s))
    }
    
    /// Associate wallet with user (non-custodial: store address only, not private key)
    pub async fn link_wallet(
        &self, 
        user_id: &str, 
        wallet_name: &str,
        wallet_address: &str,
        wallet_type: Option<&str>
    ) -> Result<()> {
        // Remove OR IGNORE to let UNIQUE constraint work properly
        let result = sqlx::query(
            "INSERT INTO user_wallets (user_id, wallet_name, wallet_address, wallet_type) VALUES (?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind(wallet_name)
        .bind(wallet_address)
        .bind(wallet_type.unwrap_or("standard"))
        .execute(&self.pool)
        .await;
        
        // Catch UNIQUE constraint error and return friendly message
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("UNIQUE constraint failed") {
                    anyhow::bail!("Wallet name already exists, please use a different name")
                } else {
                    Err(e.into())
                }
            }
        }
    }
    
    /// Get all wallet names for a user (backward compatibility)
    pub async fn get_user_wallets(&self, user_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT wallet_name FROM user_wallets WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|row| row.0).collect())
    }
    
    /// Get all wallet details for a user (including addresses)
    pub async fn get_user_wallets_with_address(&self, user_id: &str) -> Result<Vec<WalletInfo>> {
        let rows = sqlx::query_as::<_, (String, Option<String>, Option<String>, String)>(
            "SELECT wallet_name, wallet_address, wallet_type, created_at FROM user_wallets WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|row| WalletInfo {
            name: row.0,
            address: row.1,
            wallet_type: row.2.unwrap_or_else(|| "standard".to_string()),
            created_at: row.3,
        }).collect())
    }
    
    /// Delete user-wallet association (non-custodial mode)
    ///
    /// Only deletes association record from user_wallets table, doesn't affect actual blockchain wallet
    pub async fn unlink_wallet(&self, user_id: &str, wallet_name: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM user_wallets WHERE user_id = ? AND wallet_name = ?"
        )
        .bind(user_id)
        .bind(wallet_name)
        .execute(&self.pool)
        .await?;
        
        // Return whether any record was deleted
        Ok(result.rows_affected() > 0)
    }
    
    /// Update user password
    pub async fn update_password(&self, email: &str, new_password: &str) -> Result<()> {
        // Validate password strength
        if new_password.len() < 8 {
            anyhow::bail!("Password must be at least 8 characters");
        }
        
        // Hash new password
        let password_hash = Self::hash_password(new_password)?;
        
        // Update password in database
        sqlx::query("UPDATE users SET password_hash = ? WHERE email = ?")
            .bind(&password_hash)
            .bind(email)
            .execute(&self.pool)
            .await?;
        
        tracing::info!("✅ Password updated: email={}", email);
        
        Ok(())
    }
    
    // ============ Validation utility functions ============

    /// Validate email format (enhanced - maximum security)
    fn validate_email(email: &str) -> Result<()> {
        // Check length limits
        if email.len() > 255 {
            anyhow::bail!("Email length cannot exceed 255 characters");
        }
        
        // Check basic format
        if !email.contains('@') {
            anyhow::bail!("Invalid email format: missing @ symbol");
        }
        
        // Split email into local and domain parts
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid email format: incorrect number of @ symbols");
        }
        
        let local = parts[0];
        let domain = parts[1];
        
        // Validate local part
        if local.is_empty() {
            anyhow::bail!("Invalid email format: local part is empty");
        }
        
        // Validate domain part
        if domain.is_empty() || !domain.contains('.') {
            anyhow::bail!("Invalid email format: domain part is invalid");
        }
        
        // Check for special characters (simple XSS protection)
        if email.contains('<') || email.contains('>') || email.contains('"') || email.contains('\'') {
            anyhow::bail!("Invalid email format: contains illegal characters");
        }
        
        Ok(())
    }
    
    /// Validate password strength (maximum security - using validators.rs)
    fn validate_password(password: &str) -> Result<()> {
        // Use complete validation from validators.rs
        use crate::api::validators::validate_password_strength;
        
        validate_password_strength(password)
            .map_err(|(_, json)| {
                // Convert ValidationResult error to anyhow::Error
                let error_msg = &json.0.error;
                anyhow::anyhow!("{}", error_msg)
            })
    }
    
    // ============ Password hashing utility functions ============

    /// Hash password using Argon2id
    fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?
            .to_string();
        
        Ok(hash)
    }
    
    /// Verify password hash
    fn verify_password(password: &str, hash: &str) -> Result<bool> {
        tracing::info!("🔐 [verify_password] Starting password verification...");
        tracing::info!("   Input password length: {} characters", password.len());
        tracing::info!("   Input password first 5 chars: {}", &password[..password.len().min(5)]);
        tracing::info!("   Stored hash length: {} characters", hash.len());
        tracing::info!("   Stored hash first 30 chars: {}", &hash[..hash.len().min(30)]);
        
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| {
                tracing::error!("❌ Hash parsing failed: {}", e);
                anyhow::anyhow!("Invalid password hash: {}", e)
            })?;
        
        tracing::info!("   ✅ Hash parsed successfully");
        tracing::info!("   Salt: {}", parsed_hash.salt.map(|s| s.to_string()).unwrap_or("None".to_string()));
        
        let verify_result = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash);
        
        match &verify_result {
            Ok(_) => {
                tracing::info!("   ✅ Password verification successful!");
            }
            Err(e) => {
                tracing::error!("   ❌ Password verification failed: {:?}", e);
                tracing::error!("   Argon2 config used: {:?}", Argon2::default());
            }
        }
        
        Ok(verify_result.is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_password_hashing() {
        let password = "test_password_123";
        let hash = UserDatabase::hash_password(password).unwrap();
        
        assert!(UserDatabase::verify_password(password, &hash).unwrap());
        assert!(!UserDatabase::verify_password("wrong_password", &hash).unwrap());
    }
}

