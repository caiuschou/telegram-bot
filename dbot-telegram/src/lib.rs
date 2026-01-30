//! # dbot-telegram
//!
//! Telegram Bot 框架层：适配器、Bot trait 实现、最小配置、REPL 运行。
//! 仅负责 Telegram 接入与消息链执行，不包含持久化、记忆、AI 等业务逻辑。

mod adapters;
mod bot_adapter;
mod config;
mod runner;

pub use adapters::{TelegramMessageWrapper, TelegramUserWrapper};
pub use bot_adapter::TelegramBotAdapter;
pub use config::TelegramConfig;
pub use runner::run_repl;
