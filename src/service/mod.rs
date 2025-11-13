pub mod di_container;
pub mod wallet;

// Re-export WalletService to make it accessible via `crate::service::WalletService`
pub use wallet::WalletService;
