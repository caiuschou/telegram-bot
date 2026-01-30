//! 框架最小配置：仅 token、API URL、日志路径。
//! 与外部交互：从环境变量 BOT_TOKEN、TELEGRAM_API_URL、LOG_FILE 加载。

use anyhow::Result;
use std::env;

/// Telegram Bot 框架最小配置（仅 Telegram 接入与日志）。
pub struct TelegramConfig {
    pub bot_token: String,
    pub telegram_api_url: Option<String>,
    pub log_file: Option<String>,
}

impl TelegramConfig {
    /// 从环境变量加载：BOT_TOKEN 必填，TELEGRAM_API_URL、LOG_FILE 可选。
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

    /// 使用给定 token 构造，其余为 None。
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

    #[test]
    fn test_with_token() {
        let config = TelegramConfig::with_token("test_token".to_string());
        assert_eq!(config.bot_token, "test_token");
        assert!(config.telegram_api_url.is_none());
        assert!(config.log_file.is_none());
    }
}
