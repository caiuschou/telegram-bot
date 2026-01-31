//! # Telegram bot application
//!
//! Wires handler-chain, llm-handlers, handlers, and storage. Loads config from env and runs the REPL.
//! Core (Handler, Bot, Message), chain (HandlerChain), and telegram (run_repl, adapters) are merged from dbot-core, handler-chain, dbot-telegram.

pub mod chain;
pub mod cli;
pub mod components;
pub mod config;
pub mod core;
pub mod handlers;
pub mod memory;
pub mod runner;
pub mod telegram;
pub mod telegram_impl;

// Re-export CLI (integrated from dbot-cli)
pub use cli::{load_config, Cli, Commands};

// Re-export core (from dbot-core)
pub use core::{
    Bot, Handler, HandlerResponse, Message, User, Chat, MessageDirection, ToCoreMessage,
    ToCoreUser, DbotError, HandlerError, Result, init_tracing, parse_message_id, TelegramBot,
};

// Re-export chain (from handler-chain)
pub use chain::HandlerChain;

// Re-export telegram (from dbot-telegram)
pub use telegram::{
    run_repl, TelegramBotAdapter, TelegramConfig, TelegramMessageWrapper, TelegramUserWrapper,
};

pub use config::{AppExtensions, BotConfig};
pub use runner::run_bot;

pub use components::{build_bot_components, create_memory_stores, BotComponents};
pub use handlers::{
    AuthHandler, LoggingHandler, MemoryConfig, MemoryHandler, NoOpHandler, PersistenceHandler,
};
