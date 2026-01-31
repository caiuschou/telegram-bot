//! # dbot-core
//!
//! Core types and traits for the Telegram bot: [`Bot`], [`Handler`], message and user types,
//! and tracing initialization. Transport-agnostic; used by dbot-telegram and handler-chain.

pub mod bot;
pub mod error;
pub mod logger;
pub mod types;

pub use bot::{Bot, TelegramBot};
pub use error::{DbotError, HandlerError, Result};
pub use logger::init_tracing;
pub use types::{
    Chat, Handler, HandlerResponse, Message, MessageDirection, ToCoreMessage, ToCoreUser, User,
};
