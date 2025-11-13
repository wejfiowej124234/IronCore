//! 存储抽象层
//!
//! 提供user数据存储的抽象接口，支持多种存储后端

pub mod r#trait;
pub mod memory;

// 重新导出
pub use r#trait::UserStorage;
pub use memory::MemoryStorage;

