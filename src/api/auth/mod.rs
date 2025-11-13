//! 认证模块
//! 
//! 独立的user认证系统，使用独立数据库存储

pub mod user_db;

pub use user_db::{UserDatabase, User, CreateUserRequest, LoginRequest};

