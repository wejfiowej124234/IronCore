//! Ledger 硬件wallet集成
//! 
//! 此模块实现与 Ledger 硬件wallet的通信，支持：
//! - Bitcoin App
//! - Ethereum App
//! - 设备管理

#[cfg(feature = "ledger")]
pub mod apdu;
#[cfg(feature = "ledger")]
pub mod device;
#[cfg(feature = "ledger")]
pub mod transport;
#[cfg(feature = "ledger")]
pub mod bitcoin_app;
#[cfg(feature = "ledger")]
pub mod ethereum_app;

#[cfg(feature = "ledger")]
pub use device::LedgerDevice;
#[cfg(feature = "ledger")]
pub use transport::LedgerTransport;


