//! LLM handlers: inline LLM processing and mention detection.
//!
//! [`InlineLLMHandler`] runs in the handler chain: detects reply-to / @mention, builds context, calls LLM, sends reply.
//! [`LLMDetectionHandler`] only detects and sends [`LLMQuery`] to a channel (for queue-based architectures).
//! Mention detection and question extraction use [`telegram_bot::mention`].

mod inline_llm;
mod mention_detector;

pub use inline_llm::InlineLLMHandler;
pub use mention_detector::{LLMDetectionHandler, LLMQuery};
