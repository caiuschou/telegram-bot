//! BotConfig: BaseConfig + DefaultAppExtensions. Use load() for env-based loading.
//!
//! Only provides base config (Telegram + log + DB). LLM, Memory, Embedding configs
//! live in their respective crates; access via `extensions().llm_config()`, etc.

use anyhow::Result;

use super::{BaseAppExtensions, BaseConfig};

/// Bot config: BaseConfig + extensions. Use BotConfig::load() for env-based loading.
pub struct BotConfig {
    pub base: BaseConfig,
    pub extensions: BaseAppExtensions,
}

impl BotConfig {
    /// Load full config from environment variables. If `token` is provided it overrides BOT_TOKEN.
    /// Call validate() after load to check config before init.
    pub fn load(token: Option<String>) -> Result<Self> {
        let base = BaseConfig::load(token)?;
        let extensions = BaseAppExtensions::from_env()?;
        Ok(Self { base, extensions })
    }

    /// Validate config. Call after load() to fail fast before init.
    pub fn validate(&self) -> Result<()> {
        self.base.validate()
    }

    pub fn base(&self) -> &BaseConfig {
        &self.base
    }
    pub fn extensions(&self) -> &BaseAppExtensions {
        &self.extensions
    }

    // --- Base config getters (Telegram + log + DB only) ---
    pub fn bot_token(&self) -> &str {
        &self.base.bot_token
    }
    pub fn database_url(&self) -> &str {
        &self.base.database_url
    }
    pub fn log_file(&self) -> &str {
        &self.base.log_file
    }
    pub fn telegram_api_url(&self) -> Option<&str> {
        self.base.telegram_api_url.as_deref()
    }
    pub fn telegram_edit_interval_secs(&self) -> u64 {
        self.base.telegram_edit_interval_secs
    }
}
