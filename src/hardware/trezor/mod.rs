//! Trezor 硬件wallet集成
//! 
//! 此模块实现与 Trezor 硬件wallet的通信，支持：
//! - Bitcoin App
//! - Ethereum App
//! - 设备管理

#[cfg(feature = "trezor")]
pub mod messages;
#[cfg(feature = "trezor")]
pub mod transport;
#[cfg(feature = "trezor")]
pub mod device;
#[cfg(feature = "trezor")]
pub mod bitcoin_app;
#[cfg(feature = "trezor")]
pub mod ethereum_app;

#[cfg(feature = "trezor")]
pub use device::TrezorDevice;
#[cfg(feature = "trezor")]
pub use transport::TrezorTransport;

