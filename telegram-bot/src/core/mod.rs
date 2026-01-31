//! Core types and traits: Handler, Bot, Message, HandlerResponse, error, logger.
//! Merged from dbot-core; transport-agnostic.

pub mod bot;
pub mod error;
pub mod logger;
pub mod types;

pub use bot::{parse_message_id, Bot, TelegramBot};
pub use error::{DbotError, HandlerError, Result};
pub use logger::init_tracing;
pub use types::{
    Chat, Handler, HandlerResponse, Message, MessageDirection, ToCoreMessage, ToCoreUser, User,
};
