#![allow(clippy::useless_vec)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::needless_return)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::len_zero)]
#![allow(clippy::needless_range_loop)]
// src/lib.rs

pub mod api;
pub mod blockchain;
pub mod cli;
pub mod core;
pub mod crypto;
pub mod security;
pub mod shamir;
pub mod storage;
pub mod tools;

// Hardware wallet support modules
#[cfg(any(feature = "trezor", feature = "ledger"))]
pub mod hardware;

// Authentication module
pub mod auth;

// Anomaly detection module
pub mod anomaly_detection;

// Monitoring module
pub mod monitoring;
// Public module exports to ensure `defi_hot_wallet::network`, `::ops`, `::mvp` are visible in tests
pub mod mvp;
pub mod network;
pub mod ops;
// Add this export so tests can use `defi_hot_wallet::audit::...`
pub mod audit;
pub mod service;
// Add i18n export for tests
pub mod i18n;

// Conditionally compile the test environment setup. Include when running `cargo test`
// or when the explicit `test-env` feature is enabled.
#[cfg(any(test, feature = "test-env"))]
mod test_env; // This will run the ctor when the feature is enabled or during tests.
