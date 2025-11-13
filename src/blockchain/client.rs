// Basic blockchain client configuration and helpers.

use serde::{Deserialize, Serialize};

/// Configuration for a blockchain RPC client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// RPC endpoint URL (e.g. "http://localhost:8545")
    pub endpoint: String,
    /// Timeout in seconds for requests
    pub timeout: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self { endpoint: "http://localhost:8545".to_string(), timeout: 30 }
    }
}
