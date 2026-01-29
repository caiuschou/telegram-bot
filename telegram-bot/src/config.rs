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
    pub memory_lance_path: Option<String>,
    /// Embedding 服务提供商：`openai` | `zhipuai`。用于 RAG 语义检索的向量化。
    pub embedding_provider: String,
    /// 智谱 / BigModel API Key。当 `embedding_provider == "zhipuai"` 时必填。
    pub bigmodel_api_key: String,
    /// 可选：Telegram Bot API 基础 URL。设置后 Bot 请求将发往该 URL（用于测试时指向 mock 服务器）。
    /// 环境变量：`TELEGRAM_API_URL` 或 `TELOXIDE_API_URL`。
    pub telegram_api_url: Option<String>,
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
        let memory_lance_path = env::var("MEMORY_LANCE_PATH").ok();
        let embedding_provider =
            env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let bigmodel_api_key = env::var("BIGMODEL_API_KEY")
            .or_else(|_| env::var("ZHIPUAI_API_KEY"))
            .unwrap_or_default();

        let telegram_api_url = env::var("TELEGRAM_API_URL")
            .or_else(|_| env::var("TELOXIDE_API_URL"))
            .ok();

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
            memory_lance_path,
            embedding_provider,
            bigmodel_api_key,
            telegram_api_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_load_config_with_defaults() {
        // 设置必要的环境变量
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("DATABASE_URL");
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("AI_MODEL");
        env::remove_var("AI_USE_STREAMING");
        env::remove_var("AI_THINKING_MESSAGE");
        env::remove_var("MEMORY_STORE_TYPE");
        env::remove_var("MEMORY_SQLITE_PATH");
        env::remove_var("MEMORY_LANCE_PATH");
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("BIGMODEL_API_KEY");
        env::remove_var("ZHIPUAI_API_KEY");
        env::remove_var("TELEGRAM_API_URL");
        env::remove_var("TELOXIDE_API_URL");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.bot_token, "test_token");
        assert!(config.telegram_api_url.is_none());
        assert_eq!(config.database_url, "file:./telegram_bot.db");
        assert_eq!(config.log_file, "logs/telegram-bot.log");
        assert_eq!(config.openai_api_key, "test_key");
        assert_eq!(config.openai_base_url, "https://api.openai.com/v1");
        assert_eq!(config.ai_model, "gpt-3.5-turbo");
        assert_eq!(config.ai_use_streaming, false);
        assert_eq!(config.ai_thinking_message, "正在思考...");
        assert_eq!(config.memory_store_type, "memory");
        assert_eq!(config.memory_sqlite_path, "./data/memory.db");
        assert_eq!(config.embedding_provider, "openai");
        assert!(config.bigmodel_api_key.is_empty());
    }

    #[test]
    #[serial]
    fn test_load_config_with_custom_values() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "custom_token");
        env::remove_var("DATABASE_URL");
        env::set_var("DATABASE_URL", "custom.db");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "custom_key");
        env::remove_var("OPENAI_BASE_URL");
        env::set_var("OPENAI_BASE_URL", "https://custom.api.com");
        env::remove_var("AI_MODEL");
        env::set_var("AI_MODEL", "gpt-4");
        env::remove_var("AI_USE_STREAMING");
        env::set_var("AI_USE_STREAMING", "true");
        env::remove_var("AI_THINKING_MESSAGE");
        env::set_var("AI_THINKING_MESSAGE", "Thinking...");
        env::remove_var("MEMORY_STORE_TYPE");
        env::set_var("MEMORY_STORE_TYPE", "sqlite");
        env::remove_var("MEMORY_SQLITE_PATH");
        env::set_var("MEMORY_SQLITE_PATH", "/tmp/memory.db");
        env::remove_var("MEMORY_LANCE_PATH");
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("BIGMODEL_API_KEY");
        env::remove_var("ZHIPUAI_API_KEY");
        env::remove_var("TELEGRAM_API_URL");
        env::remove_var("TELOXIDE_API_URL");

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
        assert_eq!(config.embedding_provider, "openai");
    }

    #[test]
    #[serial]
    fn test_load_config_with_override_token() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "env_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("BIGMODEL_API_KEY");
        env::remove_var("ZHIPUAI_API_KEY");

        let config = BotConfig::load(Some("override_token".to_string())).unwrap();

        assert_eq!(config.bot_token, "override_token");
    }

    #[test]
    #[serial]
    fn test_load_config_embedding_provider_zhipuai() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("EMBEDDING_PROVIDER");
        env::set_var("EMBEDDING_PROVIDER", "zhipuai");
        env::remove_var("BIGMODEL_API_KEY");
        env::set_var("BIGMODEL_API_KEY", "bigmodel-key");
        env::remove_var("ZHIPUAI_API_KEY");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.embedding_provider, "zhipuai");
        assert_eq!(config.bigmodel_api_key, "bigmodel-key");
    }

    #[test]
    #[serial]
    fn test_load_config_bigmodel_key_from_zhipuai() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("BIGMODEL_API_KEY");
        env::remove_var("ZHIPUAI_API_KEY");
        env::set_var("ZHIPUAI_API_KEY", "zhipu-key");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.bigmodel_api_key, "zhipu-key");
    }
}
