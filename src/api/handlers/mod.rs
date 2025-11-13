//! API Handlers 模块
//! 
//! 按功能拆分的HTTP请求处理器

pub mod address;
pub mod backup;
pub mod balance;
pub mod bridge;
pub mod health;
pub mod multisig;
pub mod multi_assets;
pub mod system_info;
pub mod transaction;
pub mod wallet;

// 重新导出常用handlers
pub use address::get_wallet_address;
pub use backup::{backup_wallet, restore_wallet};
pub use balance::get_balance;
pub use bridge::{bridge_assets, bridge_history, bridge_status};
pub use health::{health_check, metrics};
pub use multisig::{rotate_signing_key, send_multi_sig_transaction};
pub use transaction::{
    get_transaction_history, send_transaction, transaction_status, 
    transactions_history, transactions_send
};
pub use wallet::{create_wallet, delete_wallet, list_wallets};
