// filepath: src/core/wallet/mod.rs
pub mod backup;
pub mod create;
pub mod recover;

// Re-export WalletManager for compatibility
pub use crate::core::wallet_manager::WalletManager;
