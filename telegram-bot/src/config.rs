//! Bot configuration loaded from environment variables.

use anyhow::Result;
use std::env;

/// Telegram bot config: token, DB, LLM, memory, embedding, and optional Telegram API URL. Loaded from env.
pub struct BotConfig {
    pub bot_token: String,
    pub database_url: String,
    pub log_file: String,
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub llm_model: String,
    pub llm_use_streaming: bool,
    pub llm_thinking_message: String,
    /// LLM system prompt (persona/behavior); when unset or empty, uses default. Env: `LLM_SYSTEM_PROMPT`.
    pub llm_system_prompt: Option<String>,
    pub memory_store_type: String,
    pub memory_sqlite_path: String,
    /// When true, recent messages use a dedicated SQLite store; semantic search still uses the primary store (e.g. Lance). Env: `MEMORY_RECENT_USE_SQLITE`.
    pub memory_recent_use_sqlite: bool,
    /// LanceDB path when `memory_store_type == "lance"`. Env: `MEMORY_LANCE_PATH` or `LANCE_DB_PATH` (fallback).
    pub memory_lance_path: Option<String>,
    /// Embedding provider: `openai` | `zhipuai`. Used for RAG semantic search vectorization.
    pub embedding_provider: String,
    /// BigModel (Zhipu) API key; required when `embedding_provider == "zhipuai"`.
    pub bigmodel_api_key: String,
    /// Optional Telegram Bot API base URL; when set, bot requests go there (e.g. mock server for tests). Env: `TELEGRAM_API_URL` or `TELOXIDE_API_URL`.
    pub telegram_api_url: Option<String>,
    /// Max recent messages for RAG context (RecentMessagesStrategy). Env: `MEMORY_RECENT_LIMIT`, default 10.
    pub memory_recent_limit: u32,
    /// Top-K for semantic search in RAG context (SemanticSearchStrategy). Env: `MEMORY_RELEVANT_TOP_K`, default 5.
    pub memory_relevant_top_k: u32,
    /// Min similarity score for semantic results; entries below are excluded; 0.0 = no filter. Env: `MEMORY_SEMANTIC_MIN_SCORE`, default 0.0. Recommended 0.6–0.8.
    pub memory_semantic_min_score: f32,
    /// Min interval (seconds) between edits of the same message when streaming; limits Telegram edit rate. Env: `TELEGRAM_EDIT_INTERVAL_SECS`, default 5.
    pub telegram_edit_interval_secs: u64,
}

impl BotConfig {
    /// Loads config from environment variables. If `token` is provided it overrides `BOT_TOKEN`.
    pub fn load(token: Option<String>) -> Result<Self> {
        let bot_token = token.unwrap_or_else(|| env::var("BOT_TOKEN").expect("BOT_TOKEN not set"));
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "file:./telegram_bot.db".to_string());
        let log_file = "logs/telegram-bot.log".to_string();
        let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        let openai_base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let llm_model = env::var("MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let llm_use_streaming = env::var("USE_STREAMING")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);
        let llm_thinking_message =
            env::var("THINKING_MESSAGE").unwrap_or_else(|_| "Thinking...".to_string());
        let llm_system_prompt = env::var("LLM_SYSTEM_PROMPT")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let memory_store_type =
            env::var("MEMORY_STORE_TYPE").unwrap_or_else(|_| "memory".to_string());
        let memory_sqlite_path =
            env::var("MEMORY_SQLITE_PATH").unwrap_or_else(|_| "./data/memory.db".to_string());
        let memory_recent_use_sqlite = env::var("MEMORY_RECENT_USE_SQLITE")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "1" | "true" | "yes" => Some(true),
                _ => s.parse().ok(),
            })
            .unwrap_or(false);
        let memory_lance_path = env::var("MEMORY_LANCE_PATH")
            .or_else(|_| env::var("LANCE_DB_PATH"))
            .ok();
        let embedding_provider =
            env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let bigmodel_api_key = env::var("BIGMODEL_API_KEY")
            .or_else(|_| env::var("ZHIPUAI_API_KEY"))
            .unwrap_or_default();

        let telegram_api_url = env::var("TELEGRAM_API_URL")
            .or_else(|_| env::var("TELOXIDE_API_URL"))
            .ok();

        let memory_recent_limit = env::var("MEMORY_RECENT_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        let memory_relevant_top_k = env::var("MEMORY_RELEVANT_TOP_K")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let memory_semantic_min_score = env::var("MEMORY_SEMANTIC_MIN_SCORE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let telegram_edit_interval_secs = env::var("TELEGRAM_EDIT_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        Ok(Self {
            bot_token,
            database_url,
            log_file,
            openai_api_key,
            openai_base_url,
            llm_model,
            llm_use_streaming,
            llm_thinking_message,
            llm_system_prompt,
            memory_store_type,
            memory_sqlite_path,
            memory_recent_use_sqlite,
            memory_lance_path,
            embedding_provider,
            bigmodel_api_key,
            telegram_api_url,
            memory_recent_limit,
            memory_relevant_top_k,
            memory_semantic_min_score,
            telegram_edit_interval_secs,
        })
    }

    /// Validates config combinations (e.g. EMBEDDING_PROVIDER=zhipuai requires BIGMODEL_API_KEY).
    /// Call after load() to fail fast on invalid config before initializing components.
    pub fn validate(&self) -> Result<()> {
        if self.embedding_provider.eq_ignore_ascii_case("zhipuai") && self.bigmodel_api_key.is_empty() {
            anyhow::bail!(
                "EMBEDDING_PROVIDER=zhipuai (or zhipuai) requires BIGMODEL_API_KEY or ZHIPUAI_API_KEY to be set"
            );
        }
        if let Some(ref url_str) = self.telegram_api_url {
            if reqwest::Url::parse(url_str).is_err() {
                anyhow::bail!("TELEGRAM_API_URL (or TELOXIDE_API_URL) is set but not a valid URL: {}", url_str);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_load_config_with_defaults() {
        // Set required env vars
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("DATABASE_URL");
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("MODEL");
        env::remove_var("USE_STREAMING");
        env::remove_var("THINKING_MESSAGE");
        env::remove_var("MEMORY_STORE_TYPE");
        env::remove_var("MEMORY_SQLITE_PATH");
        env::remove_var("MEMORY_LANCE_PATH");
        env::remove_var("LANCE_DB_PATH");
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("BIGMODEL_API_KEY");
        env::remove_var("ZHIPUAI_API_KEY");
        env::remove_var("TELEGRAM_API_URL");
        env::remove_var("TELOXIDE_API_URL");
        env::remove_var("MEMORY_RECENT_LIMIT");
        env::remove_var("MEMORY_RELEVANT_TOP_K");
        env::remove_var("MEMORY_RECENT_USE_SQLITE");
        env::remove_var("LLM_SYSTEM_PROMPT");
        env::remove_var("MEMORY_SEMANTIC_MIN_SCORE");
        env::remove_var("TELEGRAM_EDIT_INTERVAL_SECS");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.bot_token, "test_token");
        assert!(config.telegram_api_url.is_none());
        assert_eq!(config.database_url, "file:./telegram_bot.db");
        assert_eq!(config.log_file, "logs/telegram-bot.log");
        assert_eq!(config.openai_api_key, "test_key");
        assert_eq!(config.openai_base_url, "https://api.openai.com/v1");
        assert_eq!(config.llm_model, "gpt-3.5-turbo");
        assert_eq!(config.llm_use_streaming, false);
        assert_eq!(config.llm_thinking_message, "Thinking...");
        assert_eq!(config.memory_store_type, "memory");
        assert_eq!(config.memory_sqlite_path, "./data/memory.db");
        assert_eq!(config.embedding_provider, "openai");
        assert!(config.bigmodel_api_key.is_empty());
        assert_eq!(config.memory_recent_limit, 10);
        assert_eq!(config.memory_relevant_top_k, 5);
        assert_eq!(config.memory_recent_use_sqlite, false);
        assert_eq!(config.memory_semantic_min_score, 0.0);
        assert_eq!(config.telegram_edit_interval_secs, 5);
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
        env::remove_var("MODEL");
        env::set_var("MODEL", "gpt-4");
        env::remove_var("USE_STREAMING");
        env::set_var("USE_STREAMING", "true");
        env::remove_var("THINKING_MESSAGE");
        env::set_var("THINKING_MESSAGE", "Thinking...");
        env::remove_var("MEMORY_STORE_TYPE");
        env::set_var("MEMORY_STORE_TYPE", "sqlite");
        env::remove_var("MEMORY_SQLITE_PATH");
        env::set_var("MEMORY_SQLITE_PATH", "/tmp/memory.db");
        env::remove_var("MEMORY_LANCE_PATH");
        env::remove_var("LANCE_DB_PATH");
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("BIGMODEL_API_KEY");
        env::remove_var("ZHIPUAI_API_KEY");
        env::remove_var("TELEGRAM_API_URL");
        env::remove_var("TELOXIDE_API_URL");
        env::remove_var("MEMORY_RECENT_LIMIT");
        env::remove_var("MEMORY_RELEVANT_TOP_K");
        env::remove_var("MEMORY_RECENT_USE_SQLITE");
        env::remove_var("LLM_SYSTEM_PROMPT");
        env::remove_var("MEMORY_SEMANTIC_MIN_SCORE");
        env::set_var("TELEGRAM_EDIT_INTERVAL_SECS", "10");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.bot_token, "custom_token");
        assert_eq!(config.database_url, "custom.db");
        assert_eq!(config.telegram_edit_interval_secs, 10);
        assert_eq!(config.openai_api_key, "custom_key");
        assert_eq!(config.openai_base_url, "https://custom.api.com");
        assert_eq!(config.llm_model, "gpt-4");
        assert_eq!(config.llm_use_streaming, true);
        assert_eq!(config.llm_thinking_message, "Thinking...");
        assert_eq!(config.memory_store_type, "sqlite");
        assert_eq!(config.memory_sqlite_path, "/tmp/memory.db");
        assert_eq!(config.embedding_provider, "openai");

        env::remove_var("TELEGRAM_EDIT_INTERVAL_SECS");
    }

    #[test]
    #[serial]
    fn test_load_config_memory_recent_limit_and_top_k() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("MEMORY_RECENT_LIMIT");
        env::remove_var("MEMORY_RELEVANT_TOP_K");
        env::remove_var("LLM_SYSTEM_PROMPT");
        env::remove_var("MEMORY_SEMANTIC_MIN_SCORE");

        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.memory_recent_limit, 10);
        assert_eq!(config.memory_relevant_top_k, 5);
        assert_eq!(config.memory_semantic_min_score, 0.0);

        env::set_var("MEMORY_RECENT_LIMIT", "20");
        env::set_var("MEMORY_RELEVANT_TOP_K", "8");
        env::set_var("MEMORY_SEMANTIC_MIN_SCORE", "0.7");
        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.memory_recent_limit, 20);
        assert_eq!(config.memory_relevant_top_k, 8);
        assert_eq!(config.memory_semantic_min_score, 0.7);

        env::remove_var("MEMORY_RECENT_LIMIT");
        env::remove_var("MEMORY_RELEVANT_TOP_K");
        env::remove_var("MEMORY_SEMANTIC_MIN_SCORE");
    }

    #[test]
    #[serial]
    fn test_load_config_memory_recent_use_sqlite() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("MEMORY_RECENT_USE_SQLITE");
        env::remove_var("LLM_SYSTEM_PROMPT");

        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.memory_recent_use_sqlite, false);

        env::set_var("MEMORY_RECENT_USE_SQLITE", "1");
        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.memory_recent_use_sqlite, true);

        env::set_var("MEMORY_RECENT_USE_SQLITE", "true");
        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.memory_recent_use_sqlite, true);

        env::remove_var("MEMORY_RECENT_USE_SQLITE");
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
        env::remove_var("LLM_SYSTEM_PROMPT");

        let config = BotConfig::load(Some("override_token".to_string())).unwrap();

        assert_eq!(config.bot_token, "override_token");
    }

    #[test]
    #[serial]
    fn test_load_config_llm_system_prompt() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("LLM_SYSTEM_PROMPT");

        let config = BotConfig::load(None).unwrap();
        assert!(config.llm_system_prompt.is_none());

        env::set_var("LLM_SYSTEM_PROMPT", "You are a test persona.");
        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.llm_system_prompt.as_deref(), Some("You are a test persona."));

        // Empty or whitespace-only string → treat as unset (use default)
        env::set_var("LLM_SYSTEM_PROMPT", "");
        let config = BotConfig::load(None).unwrap();
        assert!(config.llm_system_prompt.is_none());

        env::set_var("LLM_SYSTEM_PROMPT", "   ");
        let config = BotConfig::load(None).unwrap();
        assert!(config.llm_system_prompt.is_none());

        env::remove_var("LLM_SYSTEM_PROMPT");
    }

    #[test]
    #[serial]
    fn test_load_config_memory_lance_path_fallback() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("MEMORY_LANCE_PATH");
        env::remove_var("LANCE_DB_PATH");
        env::remove_var("LLM_SYSTEM_PROMPT");

        let config = BotConfig::load(None).unwrap();
        assert!(config.memory_lance_path.is_none());

        // LANCE_DB_PATH fallback when MEMORY_LANCE_PATH is unset
        env::set_var("LANCE_DB_PATH", "./data/lancedb");
        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.memory_lance_path.as_deref(), Some("./data/lancedb"));

        // MEMORY_LANCE_PATH takes precedence over LANCE_DB_PATH
        env::set_var("MEMORY_LANCE_PATH", "./data/custom_lance");
        let config = BotConfig::load(None).unwrap();
        assert_eq!(config.memory_lance_path.as_deref(), Some("./data/custom_lance"));

        env::remove_var("MEMORY_LANCE_PATH");
        env::remove_var("LANCE_DB_PATH");
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
        env::remove_var("LLM_SYSTEM_PROMPT");

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
        env::remove_var("LLM_SYSTEM_PROMPT");

        let config = BotConfig::load(None).unwrap();

        assert_eq!(config.bigmodel_api_key, "zhipu-key");
    }

    #[test]
    #[serial]
    fn test_validate_zhipuai_requires_bigmodel_key() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("EMBEDDING_PROVIDER");
        env::set_var("EMBEDDING_PROVIDER", "zhipuai");
        env::remove_var("BIGMODEL_API_KEY");
        env::remove_var("ZHIPUAI_API_KEY");
        env::remove_var("LLM_SYSTEM_PROMPT");

        let config = BotConfig::load(None).unwrap();
        assert!(config.validate().is_err());

        env::set_var("BIGMODEL_API_KEY", "key");
        let config = BotConfig::load(None).unwrap();
        assert!(config.validate().is_ok());

        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("BIGMODEL_API_KEY");
    }

    #[test]
    #[serial]
    fn test_validate_telegram_api_url_invalid() {
        env::remove_var("BOT_TOKEN");
        env::set_var("BOT_TOKEN", "test_token");
        env::remove_var("OPENAI_API_KEY");
        env::set_var("OPENAI_API_KEY", "test_key");
        env::remove_var("TELEGRAM_API_URL");
        env::set_var("TELEGRAM_API_URL", "not-a-valid-url");
        env::remove_var("LLM_SYSTEM_PROMPT");

        let config = BotConfig::load(None).unwrap();
        assert!(config.validate().is_err());

        env::remove_var("TELEGRAM_API_URL");
    }
}
