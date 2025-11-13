//! 硬件wallet集成模块
//! 
//! 此模块提供与硬件wallet（Ledger, Trezor 等）的集成

#[cfg(feature = "ledger")]
pub mod ledger;

#[cfg(feature = "trezor")]
pub mod trezor;

#[cfg(feature = "ledger")]
pub use ledger::LedgerDevice;

#[cfg(feature = "trezor")]
pub use trezor::TrezorDevice;


