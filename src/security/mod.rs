// src/security/mod.rs
//! Security-related functionality for the wallet
//!
//! This module contains security features such as anti-debugging,
//! zeroization utilities, and other protective measures.

pub mod access_control;
pub mod compliance;
pub mod encryption;
pub mod env_manager;
pub mod memory_protection;
pub mod mnemonic_export;
pub mod password_validator;
pub mod env_validator;
pub mod error_sanitizer;
pub mod secret;
pub mod shamir;

// Add the new anti-debug module
pub mod anti_debug;

// Re-export commonly used security functions for convenience
pub use anti_debug::is_debugger_present;
pub use env_manager::{PermissionLevel, SecureEnvManager, SECURE_ENV_MANAGER};

// Secret buffer alias re-export
pub use secret::SecretVec;

// Redaction helpers to avoid accidental secret prints
pub mod redaction;
pub use redaction::{redact_body, redact_hex_bytes};
