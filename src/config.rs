use dotenvy::dotenv;
use std::env;
use thiserror::Error;

/// Error type for configuration loading and validation
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable not found: {0}")]
    MissingVar(String),

    #[error("Failed to parse environment variable '{key}': {source}")]
    ParseIntError {
        key: String,
        source: std::num::ParseIntError,
    },

    #[error("Dotenvy error: {0}")]
    Dotenv(#[from] dotenvy::Error),
}

/// Configuration manager holding all environment variables
#[derive(Debug, Clone)]
pub struct Config {
    pub rust_log: String,
    pub session_id: String,
    pub session_id_sign: String,
    pub headless: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub chart_page_id: String,
    pub use_save_shortcut: bool,
    pub request_timeout_seconds: u64,
}

impl Config {
    /// Loads configuration from the environment and optional .env file.
    pub fn new() -> Result<Self, ConfigError> {
        // Load environment variables from .env file if present
        match dotenv() {
            Err(e) if !matches!(e, dotenvy::Error::Io(_)) => return Err(ConfigError::Dotenv(e)),
            _ => {}
        }

        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        let session_id = env::var("TRADINGVIEW_SESSION_ID")
            .map_err(|_| ConfigError::MissingVar("TRADINGVIEW_SESSION_ID".to_string()))?;

        let session_id_sign = env::var("TRADINGVIEW_SESSION_ID_SIGN")
            .map_err(|_| ConfigError::MissingVar("TRADINGVIEW_SESSION_ID_SIGN".to_string()))?;

        let headless = env::var("MCP_SCRAPER_HEADLESS")
            .unwrap_or_else(|_| "true".to_string())
            .trim()
            .to_lowercase()
            == "true";

        let window_width = env::var("MCP_SCRAPER_WINDOW_WIDTH")
            .unwrap_or_else(|_| "1920".to_string())
            .parse::<u32>()
            .map_err(|source| ConfigError::ParseIntError {
                key: "MCP_SCRAPER_WINDOW_WIDTH".to_string(),
                source,
            })?;

        let window_height = env::var("MCP_SCRAPER_WINDOW_HEIGHT")
            .unwrap_or_else(|_| "1080".to_string())
            .parse::<u32>()
            .map_err(|source| ConfigError::ParseIntError {
                key: "MCP_SCRAPER_WINDOW_HEIGHT".to_string(),
                source,
            })?;

        let chart_page_id =
            env::var("MCP_SCRAPER_CHART_PAGE_ID").unwrap_or_else(|_| "".to_string());

        let use_save_shortcut = env::var("MCP_SCRAPER_USE_SAVE_SHORTCUT")
            .unwrap_or_else(|_| "true".to_string())
            .trim()
            .to_lowercase()
            == "true";

        let request_timeout_seconds = env::var("TRADESNAP_REQUEST_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "20".to_string())
            .parse::<u64>()
            .map_err(|source| ConfigError::ParseIntError {
                key: "TRADESNAP_REQUEST_TIMEOUT_SECONDS".to_string(),
                source,
            })?;

        Ok(Self {
            rust_log,
            session_id,
            session_id_sign,
            headless,
            window_width,
            window_height,
            chart_page_id,
            use_save_shortcut,
            request_timeout_seconds,
        })
    }
}
