//! 多sign管理模块
//!
//! 提供多Sign transaction的创建、sign、执行等功能
//!
//! ## 模块结构
//! - `config` - 多签配置和策略
//! - `transaction` - 多签transaction管理
//! - `signing` - sign收集和validate
//! - `policy` - 阈值策略和权限

pub mod config;
pub mod transaction;
pub mod signing;
pub mod policy;

// 重新导出核心类型
pub use config::{MultiSigConfig, AmountPrecision};
pub use transaction::{MultiSigTransaction, PendingMultiSigTransaction};
pub use signing::MultiSignature;

// 向后兼容：保留旧的导出路径
#[deprecated(since = "0.3.0", note = "Use crypto::multisig::signing::MultiSignature")]
pub use signing::MultiSignature as MultiSignatureManager;

