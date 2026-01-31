//! Config tests.

use crate::config::bot_config::BotConfig;
use crate::config::{AppExtensions, BaseAppExtensions};
use serial_test::serial;
use std::env;

#[test]
#[serial]
fn test_load_config_with_defaults() {
    env::remove_var("BOT_TOKEN");
    env::set_var("BOT_TOKEN", "test_token");
    env::remove_var("OPENAI_API_KEY");
    env::set_var("OPENAI_API_KEY", "test_key");
    env::remove_var("DATABASE_URL");
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
    env::remove_var("MEMORY_SEMANTIC_MIN_SCORE");
    env::remove_var("TELEGRAM_EDIT_INTERVAL_SECS");

    let config = BotConfig::load(None).unwrap();

    assert_eq!(config.bot_token(), "test_token");
    assert!(config.telegram_api_url().is_none());
    assert_eq!(config.database_url(), "file:./telegram_bot.db");
    assert_eq!(config.log_file(), "logs/telegram-bot.log");
    let mem = config.extensions().memory_config().unwrap();
    assert_eq!(mem.store_type(), "memory");
    assert_eq!(mem.sqlite_path(), "./data/memory.db");
    let emb = config.extensions().embedding_config().unwrap();
    assert_eq!(emb.provider(), "openai");
    assert!(emb.bigmodel_api_key().is_empty());
    assert_eq!(mem.recent_limit(), 10);
    assert_eq!(mem.relevant_top_k(), 5);
    assert_eq!(mem.recent_use_sqlite(), false);
    assert_eq!(mem.semantic_min_score(), 0.0);
    assert_eq!(config.telegram_edit_interval_secs(), 5);
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
    env::remove_var("MEMORY_SEMANTIC_MIN_SCORE");
    env::set_var("TELEGRAM_EDIT_INTERVAL_SECS", "10");

    let config = BotConfig::load(None).unwrap();

    assert_eq!(config.bot_token(), "custom_token");
    assert_eq!(config.database_url(), "custom.db");
    assert_eq!(config.telegram_edit_interval_secs(), 10);
    let mem = config.extensions().memory_config().unwrap();
    assert_eq!(mem.store_type(), "sqlite");
    assert_eq!(mem.sqlite_path(), "/tmp/memory.db");
    assert_eq!(config.extensions().embedding_config().unwrap().provider(), "openai");

    env::remove_var("TELEGRAM_EDIT_INTERVAL_SECS");
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

    assert_eq!(config.bot_token(), "override_token");
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

    let config = BotConfig::load(None).unwrap();
    assert!(config.validate().is_err());

    env::remove_var("TELEGRAM_API_URL");
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

    let result = BaseAppExtensions::from_env();
    assert!(result.is_err());

    env::remove_var("EMBEDDING_PROVIDER");
}
