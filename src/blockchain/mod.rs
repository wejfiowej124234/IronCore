pub mod audit;
pub mod bridge;
pub mod ethereum;
pub mod traits; // Added minimal stub for audit module

#[cfg(feature = "bitcoin")]
pub mod bitcoin;

pub use bridge::{BridgeTransaction, BridgeTransactionStatus};
pub use traits::{BlockchainClient, Bridge};
