//! Bot configuration: BaseConfig (Telegram + log + DB) + AppExtensions (LLM, Memory, Embedding).

mod base;
mod bot_config;
mod extensions;

#[cfg(test)]
mod tests;

pub use base::BaseConfig;
pub use bot_config::BotConfig;
pub use extensions::{AppExtensions, BaseAppExtensions};
