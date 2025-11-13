#![allow(clippy::module_inception)]
pub mod application;
pub mod service;

// Re-export key components to form the application's public API.
pub use service::Application;
