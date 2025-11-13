// src/blockchain/bridge/mod.rs

// Expose sub-modules
pub mod mock;
pub mod relay;
pub mod transfer;

use crate::core::wallet_info::SecureWalletData;
use serde::{Deserialize, Serialize};

/// Represents the status of a cross-chain bridge transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BridgeTransactionStatus {
    Initiated,
    InTransit,
    Completed,
    Failed(String),
}

/// Represents a cross-chain bridge transaction record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransaction {
    pub id: String,
    pub from_wallet: String,
    pub from_chain: String,
    pub to_chain: String,
    pub token: String,
    pub amount: String,
    pub status: BridgeTransactionStatus,
    pub source_tx_hash: Option<String>,
    pub destination_tx_hash: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub fee_amount: Option<String>,
    pub estimated_completion_time: Option<chrono::DateTime<chrono::Utc>>,
}

// Re-export commonly-used mock bridge implementations at the module root so
// tests and other code that previously imported them from
pub use mock::{
    EthereumToBSCBridge, PolygonToEthereumBridge,
};

// Re-export the Bridge trait here for compatibility with existing imports
// that expect `bridge::Bridge` to be available.
pub use crate::blockchain::traits::Bridge;

/// Thin facade to initiate bridge transfer.
pub async fn bridge_transfer(
    bridge: &dyn Bridge,
    from_chain: &str,
    to_chain: &str,
    token: &str,
    amount: &str,
    wallet_data: &SecureWalletData,
) -> anyhow::Result<String> {
    transfer::initiate_bridge_transfer(bridge, from_chain, to_chain, token, amount, wallet_data)
        .await
}

/// Thin facade to relay/check a bridge transaction.
pub async fn bridge_relay(
    bridge: &dyn Bridge,
    tx_id: &str,
) -> anyhow::Result<BridgeTransactionStatus> {
    relay::relay_transaction(bridge, tx_id).await
}
