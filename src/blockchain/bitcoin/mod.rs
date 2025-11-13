//! Bitcoin 区块链集成模块
//! 
//! 此模块提供完整的 Bitcoin 区块链支持，包括：
//! - Legacy (P2PKH) address
//! - SegWit (P2WPKH) address
//! - Taproot (P2TR) address
//! - Schnorr sign
//! - UTXO 管理
//! - transaction构建和广播

#[cfg(feature = "bitcoin")]
pub mod account;
#[cfg(feature = "bitcoin")]
pub mod address;
#[cfg(feature = "bitcoin")]
pub mod client;
#[cfg(feature = "bitcoin")]
pub mod transaction;
#[cfg(feature = "bitcoin")]
pub mod utxo;

#[cfg(feature = "bitcoin")]
pub use account::BitcoinKeypair;
#[cfg(feature = "bitcoin")]
pub use address::{AddressType, BitcoinAddress};
#[cfg(feature = "bitcoin")]
pub use client::BitcoinClient;
#[cfg(feature = "bitcoin")]
pub use transaction::BitcoinTransaction;
#[cfg(feature = "bitcoin")]
pub use utxo::{Utxo, UtxoSelector};

