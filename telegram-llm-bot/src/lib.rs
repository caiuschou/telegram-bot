//! # telegram_llm_bot
//!
//! LLM integration for Telegram bot. **Public API:** see [facade] — `run_bot_with_llm`, `run_bot_with_custom_handler`, `build_llm_handler`, `create_memory_stores_for_llm`.

mod assembly;
mod facade;
pub mod handlers;

pub use facade::*;
pub use handlers::{InlineLLMHandler, LLMDetectionHandler, LLMQuery};
