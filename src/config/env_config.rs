// ...existing code...
use anyhow::Result;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct AppEnvConfig {
    /// Database URL (uses DATABASE_URL env or falls back to sqlite file)
    pub database_url: String,
    /// Optional Ethereum RPC URL (WALLET_ETHEREUM_RPC_URL)
    pub ethereum_rpc_url: Option<String>,
    /// Optional additional config fields used by the app
    pub some_field: Option<String>,
    pub another_field: Option<String>,
}

impl AppEnvConfig {
    pub fn from_env() -> Result<Self> {
        use defi_hot_wallet::security::env_manager::secure_env;

        let database_url = secure_env::get_database_url()
            .unwrap_or_else(|_| "sqlite://./wallets.db".to_string());
        let ethereum_rpc_url = secure_env::get_ethereum_rpc_url().ok();
        let some_field = std::env::var("APP_SOME_FIELD").ok(); // Keep as-is for non-sensitive fields
        let another_field = std::env::var("APP_ANOTHER_FIELD").ok(); // Keep as-is for non-sensitive fields

        Ok(AppEnvConfig { database_url, ethereum_rpc_url, some_field, another_field })
    }
}
// ...existing code...
