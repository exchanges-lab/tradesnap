pub mod config;
pub mod scraper;
pub mod structs;

// Re-export the config module interfaces for easier consumption
pub use config::{Config, ConfigError};
pub use scraper::TradingViewScraper;
