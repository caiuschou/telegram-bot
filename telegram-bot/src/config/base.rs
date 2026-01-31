//! Base config: Telegram Bot connection, logging, database. Loaded from env.

use anyhow::Result;
use std::env;

/// Base config: Telegram-related, logging, database only.
#[derive(Debug, Clone)]
pub struct BaseConfig {
    /// BOT_TOKEN
    pub bot_token: String,
    /// TELEGRAM_API_URL or TELOXIDE_API_URL
    pub telegram_api_url: Option<String>,
    /// Min interval (sec) between message edits when streaming; limits Telegram API rate
    pub telegram_edit_interval_secs: u64,
    /// Log file path
    pub log_file: String,
    /// Message persistence database URL (SQLite file: or PostgreSQL etc.)
    pub database_url: String,
}

impl BaseConfig {
    /// Load from environment variables. `token` overrides BOT_TOKEN if provided.
    pub fn load(token: Option<String>) -> Result<Self> {
        let bot_token = token
            .unwrap_or_else(|| env::var("BOT_TOKEN").expect("BOT_TOKEN not set"));
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "file:./telegram_bot.db".to_string());
        let log_file =
            env::var("LOG_FILE").unwrap_or_else(|_| "logs/telegram-bot.log".to_string());
        let telegram_api_url = env::var("TELEGRAM_API_URL")
            .or_else(|_| env::var("TELOXIDE_API_URL"))
            .ok();
        let telegram_edit_interval_secs = env::var("TELEGRAM_EDIT_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        Ok(Self {
            bot_token,
            telegram_api_url,
            telegram_edit_interval_secs,
            log_file,
            database_url,
        })
    }

    /// Validate config (e.g. telegram_api_url must be valid URL if set).
    pub fn validate(&self) -> Result<()> {
        if let Some(ref url_str) = self.telegram_api_url {
            if reqwest::Url::parse(url_str).is_err() {
                anyhow::bail!(
                    "TELEGRAM_API_URL (or TELOXIDE_API_URL) is set but not a valid URL: {}",
                    url_str
                );
            }
        }
        Ok(())
    }
}
