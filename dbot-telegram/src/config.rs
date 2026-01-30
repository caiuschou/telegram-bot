//! Minimal framework config: token, optional API URL and log file path. Loaded from env: BOT_TOKEN, TELEGRAM_API_URL (or TELOXIDE_API_URL), LOG_FILE.

use anyhow::Result;
use std::env;

/// Minimal Telegram bot config (connectivity and logging only).
pub struct TelegramConfig {
    pub bot_token: String,
    pub telegram_api_url: Option<String>,
    pub log_file: Option<String>,
}

impl TelegramConfig {
    /// Loads from env: BOT_TOKEN required; TELEGRAM_API_URL and LOG_FILE optional.
    pub fn from_env() -> Result<Self> {
        let bot_token = env::var("BOT_TOKEN").map_err(|_| anyhow::anyhow!("BOT_TOKEN not set"))?;
        let telegram_api_url = env::var("TELEGRAM_API_URL")
            .or_else(|_| env::var("TELOXIDE_API_URL"))
            .ok();
        let log_file = env::var("LOG_FILE").ok();
        Ok(Self {
            bot_token,
            telegram_api_url,
            log_file,
        })
    }

    /// Builds config with the given token; other fields None.
    pub fn with_token(bot_token: String) -> Self {
        Self {
            bot_token,
            telegram_api_url: None,
            log_file: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Test: with_token sets bot_token; telegram_api_url and log_file are None.**
    #[test]
    fn test_with_token() {
        let config = TelegramConfig::with_token("test_token".to_string());
        assert_eq!(config.bot_token, "test_token");
        assert!(config.telegram_api_url.is_none());
        assert!(config.log_file.is_none());
    }
}
