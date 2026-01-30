//! # LLM handlers for dbot
//!
//! [`LLMDetectionHandler`] detects @-mentions and replies-to-bot and sends [`LLMQuery`] to a channel.
//! [`SyncLLMHandler`] consumes queries, builds context, calls the LLM, and sends replies (optionally streamed).

mod llm_mention_detector;
mod sync_llm_handler;

pub use llm_mention_detector::{LLMDetectionHandler, LLMQuery};
pub use sync_llm_handler::SyncLLMHandler;
