//! OAuth提供商插件层

pub mod r#trait;
pub mod google;

// 重新导出
pub use r#trait::OAuthProvider;
pub use google::GoogleProvider;

