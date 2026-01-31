//! # Telegram bot application
//!
//! Wires dbot-telegram, handler-chain, llm-handlers, middleware, and storage. Loads config from env and runs the REPL.

pub mod components;
pub mod config;
pub mod runner;
pub mod telegram_impl;

pub use dbot_telegram::{TelegramMessageWrapper, TelegramUserWrapper};
pub use config::{AppExtensions, BotConfig};
pub use runner::run_bot;
pub use telegram_impl::TelegramBot;

pub use components::{build_bot_components, create_memory_stores, BotComponents};
