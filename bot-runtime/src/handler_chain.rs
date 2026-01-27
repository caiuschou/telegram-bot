use dbot_core::{Handler, HandlerResponse, Message, Result};
use std::sync::Arc;
use tracing::{debug, info, instrument};

#[derive(Clone)]
pub struct HandlerChain {
    handlers: Vec<Arc<dyn Handler>>,
}

impl HandlerChain {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(mut self, handler: Arc<dyn Handler>) -> Self {
        self.handlers.push(handler);
        self
    }

    #[instrument(skip(self, message))]
    pub async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let mut final_response = HandlerResponse::Continue;

        for handler in &self.handlers {
            let response = handler.handle(message).await?;
            debug!(
                handler = std::any::type_name_of_val(handler.as_ref()),
                response = ?response,
                "Handler processed"
            );

            match response {
                HandlerResponse::Stop => {
                    info!("Handler chain stopped");
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

        Ok(final_response)
    }
}
