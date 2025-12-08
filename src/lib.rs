pub mod config;
pub mod db;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod utils;

pub use config::Config;

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");