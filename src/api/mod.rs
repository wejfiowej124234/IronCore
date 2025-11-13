// src/api/mod.rs

pub mod handlers;
pub mod middleware;     // Authentication and other middleware
pub mod server;
pub mod server_config;  // Server configuration constants
pub mod csp_middleware; // CSP security policy middleware
pub mod types;
pub mod validators;     // Shared validation logic
pub mod anomaly_detection;
// pub mod auth;        // Old version (temporarily disabled)
pub mod auth_simple;    // Simplified authentication module
pub mod user_db;        // Separate user database module
pub mod session_store;  // Session storage module
pub mod user_preferences; // User preferences module
pub mod swap;           // DEX aggregation swap module
pub mod nft;            // NFT asset management module
pub mod gamefi;         // GameFi and airdrop management module
pub mod bridge_lifi;    // LI.FI cross-chain bridge
pub mod bridge_enhanced; // Enhanced cross-chain bridge

// Re-export commonly used types
pub use crate::core::wallet_info::WalletInfo;
