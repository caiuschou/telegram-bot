//! # dbot-cli
//!
//! Base CLI foundation: argument parsing, config loading. No LLM logic.

pub mod cli;

pub use cli::{load_config, Cli, Commands};
pub use telegram_bot::BotConfig;
