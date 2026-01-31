//! # Handler chain
//!
//! Runs a sequence of handlers. Each handler has optional before/handle/after: all before run in
//! order (any false stops the chain); then handle runs until Stop or Reply; then all after run in reverse.

use crate::core::{Handler, HandlerResponse, Message, Result};
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// Chain of handlers: before (all) → handle (until Stop/Reply) → after (reverse).
#[derive(Clone)]
pub struct HandlerChain {
    handlers: Vec<Arc<dyn Handler>>,
}

impl HandlerChain {
    /// Creates an empty chain.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Appends a handler.
    pub fn add_handler(mut self, handler: Arc<dyn Handler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Runs all before → handle until Stop/Reply → all after in reverse.
    #[instrument(skip(self, message))]
    pub async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let mut final_response = HandlerResponse::Continue;

        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            message_id = %message.id,
            "step: handler_chain started"
        );

        for h in &self.handlers {
            let name = std::any::type_name_of_val(h.as_ref());
            info!(user_id = message.user.id, handler = %name, "step: handler before");
            let should_continue = h.before(message).await?;
            if !should_continue {
                info!(user_id = message.user.id, handler = %name, "step: before returned false, chain stopped");
                return Ok(HandlerResponse::Stop);
            }
            info!(user_id = message.user.id, handler = %name, "step: handler before done");
        }

        for h in &self.handlers {
            let name = std::any::type_name_of_val(h.as_ref());
            info!(user_id = message.user.id, handler = %name, "step: handler handle");
            let response = h.handle(message).await?;
            debug!(handler = %name, response = ?response, "Handler processed");
            let (response_type, reply_len) = match &response {
                HandlerResponse::Continue => ("Continue", None),
                HandlerResponse::Stop => ("Stop", None),
                HandlerResponse::Ignore => ("Ignore", None),
                HandlerResponse::Reply(s) => ("Reply", Some(s.len())),
            };
            info!(
                user_id = message.user.id,
                handler = %name,
                response_type = %response_type,
                reply_len = ?reply_len,
                "step: handler handle done"
            );

            match response {
                HandlerResponse::Stop | HandlerResponse::Reply(_) => {
                    info!(user_id = message.user.id, "step: handler chain stopped by handler");
                    final_response = response;
                    break;
                }
                HandlerResponse::Continue | HandlerResponse::Ignore => {}
            }
        }

        for h in self.handlers.iter().rev() {
            let name = std::any::type_name_of_val(h.as_ref());
            info!(user_id = message.user.id, handler = %name, "step: handler after");
            h.after(message, &final_response).await?;
            info!(user_id = message.user.id, handler = %name, "step: handler after done");
        }

        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            message_id = %message.id,
            "step: handler_chain finished"
        );

        Ok(final_response)
    }
}

// Unit/integration tests live in tests/handler_chain_test.rs
