//! Core types: user, chat, message, handler response, and Handler trait.
//!
//! Types are split into one file per main type for easier navigation and alignment with project conventions.

mod chat;
mod handler;
mod message;
mod response;
mod user;

pub use chat::Chat;
pub use handler::{Handler, ToCoreMessage, ToCoreUser};
pub use message::{Message, MessageDirection};
pub use response::HandlerResponse;
pub use user::User;
