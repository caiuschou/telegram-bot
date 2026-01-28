use dbot_core::{Handler, HandlerResponse, Message, Middleware, Result};
use std::sync::Arc;
use tracing::{debug, info, instrument};

#[derive(Clone)]
pub struct HandlerChain {
    middleware: Vec<Arc<dyn Middleware>>,
    handlers: Vec<Arc<dyn Handler>>,
}

impl HandlerChain {
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
            handlers: Vec::new(),
        }
    }

    pub fn add_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    pub fn add_handler(mut self, handler: Arc<dyn Handler>) -> Self {
        self.handlers.push(handler);
        self
    }

    #[instrument(skip(self, message))]
    pub async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let mut final_response = HandlerResponse::Continue;

        for mw in &self.middleware {
            let should_continue = mw.before(message).await?;
            if !should_continue {
                info!("Middleware chain stopped before handler execution");
                return Ok(HandlerResponse::Stop);
            }
        }

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

        for mw in self.middleware.iter().rev() {
            mw.after(message, &final_response).await?;
        }

        Ok(final_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbot_core::{User, Chat};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use chrono::Utc;

    fn create_test_message(content: &str) -> Message {
        Message {
            id: "test_message_id".to_string(),
            content: content.to_string(),
            user: User {
                id: 123,
                username: Some("test_user".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 456,
                chat_type: "private".to_string(),
            },
            message_type: "text".to_string(),
            direction: dbot_core::MessageDirection::Incoming,
            created_at: Utc::now(),
            reply_to_message_id: None,
        }
    }

    struct TestMiddleware {
        before_count: Arc<AtomicUsize>,
        after_count: Arc<AtomicUsize>,
    }

    impl TestMiddleware {
        fn new(before_count: Arc<AtomicUsize>, after_count: Arc<AtomicUsize>) -> Self {
            Self { before_count, after_count }
        }
    }

    #[async_trait::async_trait]
    impl Middleware for TestMiddleware {
        async fn before(&self, _message: &Message) -> Result<bool> {
            self.before_count.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        }

        async fn after(&self, _message: &Message, _response: &HandlerResponse) -> Result<()> {
            self.after_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct TestHandler {
        handle_count: Arc<AtomicUsize>,
    }

    impl TestHandler {
        fn new(handle_count: Arc<AtomicUsize>) -> Self {
            Self { handle_count }
        }
    }

    #[async_trait::async_trait]
    impl Handler for TestHandler {
        async fn handle(&self, _message: &Message) -> Result<HandlerResponse> {
            self.handle_count.fetch_add(1, Ordering::SeqCst);
            Ok(HandlerResponse::Continue)
        }
    }

    #[tokio::test]
    async fn test_handler_chain_with_middleware() {
        let before_count = Arc::new(AtomicUsize::new(0));
        let after_count = Arc::new(AtomicUsize::new(0));
        let handle_count = Arc::new(AtomicUsize::new(0));

        let middleware = Arc::new(TestMiddleware::new(before_count.clone(), after_count.clone()));
        let handler = Arc::new(TestHandler::new(handle_count.clone()));

        let chain = HandlerChain::new()
            .add_middleware(middleware)
            .add_handler(handler);

        let message = create_test_message("test");
        chain.handle(&message).await.unwrap();

        assert_eq!(before_count.load(Ordering::SeqCst), 1);
        assert_eq!(handle_count.load(Ordering::SeqCst), 1);
        assert_eq!(after_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_middleware_stops_chain() {
        struct BlockingMiddleware;

        #[async_trait::async_trait]
        impl Middleware for BlockingMiddleware {
            async fn before(&self, _message: &Message) -> Result<bool> {
                Ok(false)
            }

            async fn after(&self, _message: &Message, _response: &HandlerResponse) -> Result<()> {
                Ok(())
            }
        }

        let handle_count = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(TestHandler::new(handle_count.clone()));

        let chain = HandlerChain::new()
            .add_middleware(Arc::new(BlockingMiddleware))
            .add_handler(handler);

        let message = create_test_message("test");
        let result = chain.handle(&message).await.unwrap();

        assert_eq!(result, HandlerResponse::Stop);
        assert_eq!(handle_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_multiple_middleware_executed_in_order() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        struct OrderMiddleware {
            name: String,
            order: Arc<std::sync::Mutex<Vec<String>>>,
        }

        #[async_trait::async_trait]
        impl Middleware for OrderMiddleware {
            async fn before(&self, _message: &Message) -> Result<bool> {
                self.order.lock().unwrap().push(format!("before_{}", self.name));
                Ok(true)
            }

            async fn after(&self, _message: &Message, _response: &HandlerResponse) -> Result<()> {
                self.order.lock().unwrap().push(format!("after_{}", self.name));
                Ok(())
            }
        }

        let chain = HandlerChain::new()
            .add_middleware(Arc::new(OrderMiddleware {
                name: "first".to_string(),
                order: order.clone(),
            }))
            .add_middleware(Arc::new(OrderMiddleware {
                name: "second".to_string(),
                order: order.clone(),
            }));

        let message = create_test_message("test");
        chain.handle(&message).await.unwrap();

        let executed = order.lock().unwrap();
        assert_eq!(
            *executed,
            vec![
                "before_first",
                "before_second",
                "after_second",
                "after_first"
            ]
        );
    }
}
