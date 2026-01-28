use anyhow::Result;
use std::env;

/// Telegram Bot 配置，从环境变量加载
pub struct BotConfig {
    pub bot_token: String,
    pub database_url: String,
    pub log_file: String,
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub ai_model: String,
    pub ai_use_streaming: bool,
    pub ai_thinking_message: String,
    pub memory_store_type: String,
    pub memory_sqlite_path: String,
}

impl BotConfig {
    /// 从环境变量加载配置
    /// 如果未指定 token，则从环境变量读取，否则使用传入的值
    pub fn load(token: Option<String>) -> Result<Self> {
        let bot_token = token.unwrap_or_else(|| env::var("BOT_TOKEN").expect("BOT_TOKEN not set"));
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "file:./telegram_bot.db".to_string());
        let log_file = "logs/telegram-bot.log".to_string();
        let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        let openai_base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let ai_model = env::var("AI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let ai_use_streaming = env::var("AI_USE_STREAMING")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);
        let ai_thinking_message =
            env::var("AI_THINKING_MESSAGE").unwrap_or_else(|_| "正在思考...".to_string());
        let memory_store_type =
            env::var("MEMORY_STORE_TYPE").unwrap_or_else(|_| "memory".to_string());
        let memory_sqlite_path =
            env::var("MEMORY_SQLITE_PATH").unwrap_or_else(|_| "./data/memory.db".to_string());

        Ok(Self {
            bot_token,
            database_url,
            log_file,
            openai_api_key,
            openai_base_url,
            ai_model,
            ai_use_streaming,
            ai_thinking_message,
            memory_store_type,
            memory_sqlite_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_with_defaults() {
        // 设置必要的环境变量
        env::set_var("BOT_TOKEN", "test_token");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("DATABASE_URL");
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("AI_MODEL");
        env::remove_var("AI_USE_STREAMING");
        env::remove_var("AI_THINKING_MESSAGE");
        env::remove_var("MEMORY_STORE_TYPE");
        env::remove_var("MEMORY_SQLITE_PATH");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.bot_token, "test_token");
        assert_eq!(config.database_url, "file:./telegram_bot.db");
        assert_eq!(config.log_file, "logs/telegram-bot.log");
        assert_eq!(config.openai_api_key, "test_key");
        assert_eq!(config.openai_base_url, "https://api.openai.com/v1");
        assert_eq!(config.ai_model, "gpt-3.5-turbo");
        assert_eq!(config.ai_use_streaming, false);
        assert_eq!(config.ai_thinking_message, "正在思考...");
        assert_eq!(config.memory_store_type, "memory");
        assert_eq!(config.memory_sqlite_path, "./data/memory.db");
    }

    #[test]
    fn test_load_config_with_custom_values() {
        env::set_var("BOT_TOKEN", "custom_token");
        env::set_var("DATABASE_URL", "custom.db");
        env::set_var("OPENAI_API_KEY", "custom_key");
        env::set_var("OPENAI_BASE_URL", "https://custom.api.com");
        env::set_var("AI_MODEL", "gpt-4");
        env::set_var("AI_USE_STREAMING", "true");
        env::set_var("AI_THINKING_MESSAGE", "Thinking...");
        env::set_var("MEMORY_STORE_TYPE", "sqlite");
        env::set_var("MEMORY_SQLITE_PATH", "/tmp/memory.db");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.bot_token, "custom_token");
        assert_eq!(config.database_url, "custom.db");
        assert_eq!(config.openai_api_key, "custom_key");
        assert_eq!(config.openai_base_url, "https://custom.api.com");
        assert_eq!(config.ai_model, "gpt-4");
        assert_eq!(config.ai_use_streaming, true);
        assert_eq!(config.ai_thinking_message, "Thinking...");
        assert_eq!(config.memory_store_type, "sqlite");
        assert_eq!(config.memory_sqlite_path, "/tmp/memory.db");
    }

    #[test]
    fn test_load_config_with_override_token() {
        env::set_var("BOT_TOKEN", "env_token");
        env::set_var("OPENAI_API_KEY", "test_key");

        let config = BotConfig::load(Some("override_token".to_string())).unwrap();

        assert_eq!(config.bot_token, "override_token");
    }
}
