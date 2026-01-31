//! No-op handler: always returns Continue. Used as terminal handler for base bot (no LLM).

use crate::core::{Handler, HandlerResponse, Message, Result};
use async_trait::async_trait;

/// Handler that does nothing; always continues. Used when running base bot without LLM.
#[derive(Clone)]
pub struct NoOpHandler;

impl NoOpHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Handler for NoOpHandler {
    async fn handle(&self, _message: &Message) -> Result<HandlerResponse> {
        Ok(HandlerResponse::Continue)
    }
}
