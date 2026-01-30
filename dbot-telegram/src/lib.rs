//! # dbot-telegram
//!
//! Telegram bot framework layer: adapters, [`dbot_core::Bot`] implementation, minimal config, REPL runner.
//! Handles only Telegram connectivity and handler-chain execution; no persistence, memory, or AI logic.

mod adapters;
mod bot_adapter;
mod config;
mod runner;

pub use adapters::{TelegramMessageWrapper, TelegramUserWrapper};
pub use bot_adapter::TelegramBotAdapter;
pub use config::TelegramConfig;
pub use runner::run_repl;
