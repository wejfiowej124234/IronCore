//! PayGate测试模块
//!
//! 全面覆盖支付网关相关的错误分支和边界情况
//! 目标：提升覆盖率到70-75%

pub mod transaction_failures;
pub mod network_errors;
pub mod auth_and_replay;
pub mod signature_verification;
pub mod deposit_failures;
pub mod withdrawal_failures;
pub mod chain_congestion;
pub mod user_rejections;

