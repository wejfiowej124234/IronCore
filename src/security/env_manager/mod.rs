//! 安全环境管理模块
//!
//! 提供安全环境变量管理和权限控制
//!
//! ## 模块结构
//! - `manager` - SecureEnvManager 核心
//! - `permissions` - 权限级别和控制
//! - `secure_env` - 安全环境变量访问
//! - `validation` - 环境validate

pub mod manager;
pub mod permissions;
pub mod secure_env;
pub mod validation;

// 重新导出核心类型
pub use manager::SecureEnvManager;
pub use permissions::PermissionLevel;
pub use secure_env::*;

// 全局单例（向后兼容）
pub use manager::SECURE_ENV_MANAGER;

