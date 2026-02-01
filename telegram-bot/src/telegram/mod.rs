//! Telegram framework layer: adapters, Bot implementation, minimal config, REPL runner.
//! Merged from dbot-telegram.

mod adapters;
mod bot_adapter;
mod config;
mod runner;

pub use adapters::{TelegramMessageWrapper, TelegramUserWrapper};
pub use bot_adapter::TelegramBotAdapter;
pub use config::TelegramConfig;
pub use runner::run_repl;
