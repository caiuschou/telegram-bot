//! # Handler chain
//!
//! Runs a sequence of middleware (before/after) and handlers for each message. Middleware can stop
//! the chain; the first handler that returns Stop or Reply ends handler execution; after callbacks run in reverse order.

use dbot_core::{Handler, HandlerResponse, Message, Middleware, Result};
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// Chain of middleware and handlers: middleware run in order (before), then handlers; middleware after run in reverse order.
#[derive(Clone)]
pub struct HandlerChain {
    middleware: Vec<Arc<dyn Middleware>>,
    handlers: Vec<Arc<dyn Handler>>,
}

impl HandlerChain {
    /// Creates an empty chain (no middleware, no handlers).
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
            handlers: Vec::new(),
        }
    }

    /// Appends a middleware (runs before handlers, after in reverse).
    pub fn add_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Appends a handler (runs in order; first Stop/Reply ends handler phase).
    pub fn add_handler(mut self, handler: Arc<dyn Handler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Runs middleware before, then handlers; then middleware after in reverse. Returns first Stop or Reply, or Continue.
    #[instrument(skip(self, message))]
    pub async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let mut final_response = HandlerResponse::Continue;

        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            message_id = %message.id,
            "step: handler_chain started"
        );

        // Run all middleware before; if any returns false, stop and return Stop.
        for mw in &self.middleware {
            let mw_name = std::any::type_name_of_val(mw.as_ref());
            info!(
                user_id = message.user.id,
                middleware = %mw_name,
                "step: middleware before"
            );
            let should_continue = mw.before(message).await?;
            if !should_continue {
                info!(
                    user_id = message.user.id,
                    middleware = %mw_name,
                    "step: middleware before returned false, chain stopped"
                );
                return Ok(HandlerResponse::Stop);
            }
            info!(
                user_id = message.user.id,
                middleware = %mw_name,
                "step: middleware before done"
            );
        }

        for handler in &self.handlers {
            let handler_name = std::any::type_name_of_val(handler.as_ref());
            info!(
                user_id = message.user.id,
                handler = %handler_name,
                "step: handler processing"
            );
            let response = handler.handle(message).await?;
            debug!(
                handler = %handler_name,
                response = ?response,
                "Handler processed"
            );
            let (response_type, reply_len) = match &response {
                HandlerResponse::Continue => ("Continue", None),
                HandlerResponse::Stop => ("Stop", None),
                HandlerResponse::Ignore => ("Ignore", None),
                HandlerResponse::Reply(s) => ("Reply", Some(s.len())),
            };
            info!(
                user_id = message.user.id,
                handler = %handler_name,
                response_type = %response_type,
                reply_len = ?reply_len,
                "step: handler done"
            );

            match response {
                HandlerResponse::Stop | HandlerResponse::Reply(_) => {
                    info!(
                        user_id = message.user.id,
                        "step: handler chain stopped by handler"
                    );
                    final_response = response;
                    break;
                }
                HandlerResponse::Continue => {
                    continue;
                }
                HandlerResponse::Ignore => {
                    continue;
                }
            }
        }

        // Run middleware after in reverse order (last added runs first here).
        for mw in self.middleware.iter().rev() {
            let mw_name = std::any::type_name_of_val(mw.as_ref());
            info!(
                user_id = message.user.id,
                middleware = %mw_name,
                "step: middleware after"
            );
            mw.after(message, &final_response).await?;
            info!(
                user_id = message.user.id,
                middleware = %mw_name,
                "step: middleware after done"
            );
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
