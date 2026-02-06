//! LLM handlers: sync LLM processing and mention detection.
//!
//! [`SyncLLMHandler`] runs in the handler chain: detects reply-to / @mention, builds context, calls LLM, sends reply.
//! [`LLMDetectionHandler`] only detects and sends [`LLMQuery`] to a channel (for queue-based architectures).
//! Mention detection and question extraction use [`telegram_bot::mention`].

mod mention_detector;
mod sync_llm;

pub use mention_detector::{LLMDetectionHandler, LLMQuery};
pub use sync_llm::SyncLLMHandler;
