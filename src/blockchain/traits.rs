use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    blockchain::bridge::BridgeTransactionStatus, core::errors::WalletError,
    core::wallet_info::SecureWalletData,
};

/// Defines the interface for a cross-chain bridge.
#[async_trait]
pub trait Bridge: Send + Sync {
    async fn check_transfer_status(&self, tx_id: &str) -> anyhow::Result<BridgeTransactionStatus>;
    async fn transfer_across_chains(
        &self,
        from_chain: &str,
        to_chain: &str,
        token: &str,
        amount: &str,
        wallet_data: &SecureWalletData,
    ) -> anyhow::Result<String>;
}

/// Represents the status of a standard blockchain transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Unknown,
}

/// Defines the standard interface for interacting with a blockchain.
#[async_trait]
pub trait BlockchainClient: Send + Sync {
    /// Clones the client into a boxed trait object.
    fn clone_box(&self) -> Box<dyn BlockchainClient>;

    /// Retrieves the balance of a given address.
    async fn get_balance(&self, address: &str) -> Result<String, WalletError>;

    /// Sends a transaction using the provided private key to a recipient address.
    async fn send_transaction(
        &self,
        private_key: &crate::core::domain::PrivateKey,
        to_address: &str,
        amount: &str,
    ) -> Result<String, WalletError>;

    /// Sends a transaction but allows the caller to provide a specific nonce.
    /// Default implementation falls back to `send_transaction` for clients that
    /// don't need/implement explicit nonce control.
    async fn send_transaction_with_nonce(
        &self,
        private_key: &crate::core::domain::PrivateKey,
        to_address: &str,
        amount: &str,
        nonce: Option<u64>,
    ) -> Result<String, WalletError> {
        // Default to the existing behavior if the client doesn't override.
        let _ = nonce; // silence unused variable if default is used
        self.send_transaction(private_key, to_address, amount).await
    }

    /// Retrieves the status of a transaction given its hash.
    async fn get_transaction_status(&self, tx_hash: &str)
        -> Result<TransactionStatus, WalletError>;

    /// Estimates the fee for a transaction.
    async fn estimate_fee(&self, to_address: &str, amount: &str) -> Result<String, WalletError>;

    /// Gets the current nonce for an address.
    async fn get_nonce(&self, address: &str) -> Result<u64, WalletError>;

    /// Gets the current block number.
    async fn get_block_number(&self) -> Result<u64, WalletError>;

    /// Validates if a given address string is valid for the blockchain.
    fn validate_address(&self, address: &str) -> anyhow::Result<bool>;

    /// Returns the name of the network (e.g., "ethereum", "polygon-testnet").
    fn get_network_name(&self) -> &str;

    /// Returns the symbol of the native token (e.g., "ETH", "MATIC").
    fn get_native_token(&self) -> &str;
}

/// Basic information about a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: String,
}

/// Generic transaction type for blockchain operations.
pub type Transaction = TransactionInfo;
