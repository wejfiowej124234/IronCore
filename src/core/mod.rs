pub mod abi;
pub mod config;
pub mod domain;
pub mod errors;
pub mod result_ext;  // Result扩展工具
pub mod key_management;
pub mod key_manager;
pub mod memory_protection;
pub mod validation;
pub mod wallet;
pub mod wallet_info;
pub mod wallet_manager;

// 閲嶆柊瀵煎嚭鍏抽敭缁撴瀯
pub use wallet_info::{SecureWalletData, WalletInfo};
pub use wallet_manager::WalletManager;

// Test-only helper modules for HD derivation probes/vectors
#[cfg(test)]
mod wallet_manager_bip44_tests;
