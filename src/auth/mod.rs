//! user认证模块（Level 5 企业级架构）
//!
//! ## 架构设计
//!
//! ```text
//! auth/
//! ├── types.rs          # 类型定义
//! ├── errors.rs         # error类型
//! ├── config.rs         # 配置管理
//! ├── service.rs        # 认证服务（门面）
//! ├── core/             # 核心业务逻辑层
//! │   ├── user_service.rs
//! │   ├── token_service.rs
//! │   └── password_service.rs
//! ├── providers/        # OAuth提供商插件层
//! │   ├── trait.rs
//! │   └── google.rs
//! ├── storage/          # 存储抽象层
//! │   ├── trait.rs
//! │   └── memory.rs
//! └── api/              # API接口层
//!     ├── routes.rs
//!     └── handlers.rs
//! ```
//!
//! ## 设计原则
//!
//! - **分层架构**: API → 服务 → 存储
//! - **依赖注入**: 通过trait实现解耦
//! - **单一职责**: 每个模块只负责一件事
//! - **开放封闭**: 易于扩展，不易修改
//! - **接口隔离**: 使用trait定义清晰的接口

pub mod types;
pub mod errors;
pub mod config;
pub mod service;
pub mod core;
pub mod providers;
pub mod storage;
pub mod api;

// 重新导出常用类型和函数
pub use types::{User, RegisterRequest, LoginRequest, AuthResponse, UserStatus};
pub use errors::AuthError;
pub use config::AuthConfig;
pub use service::AuthService;
pub use api::create_auth_routes;
pub use storage::{UserStorage, MemoryStorage};

/// 认证模块版本
pub const VERSION: &str = "1.0.0";

/// 认证模块架构等级
pub const ARCHITECTURE_LEVEL: u8 = 5;

