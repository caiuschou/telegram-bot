//! Integration tests for [`handler_chain::HandlerChain`].
//!
//! Covers: middleware before/after order, middleware stopping the chain, handler Reply stopping the chain
//! and being passed to middleware after, and multiple middleware executed in order (before first→last, after last→first).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::Utc;
use dbot_core::{Chat, Handler, HandlerResponse, Message, MessageDirection, Middleware, User};
use handler_chain::HandlerChain;

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
        direction: MessageDirection::Incoming,
        created_at: Utc::now(),
        reply_to_message_id: None,
        reply_to_message_from_bot: false,
        reply_to_message_content: None,
    }
}

/// **Test: Middleware before and after run; handler runs once.**
///
/// **Setup:** One middleware (counts before/after), one handler (counts handle).
/// **Action:** `chain.handle(&message)`.
/// **Expected:** before_count=1, handle_count=1, after_count=1; response is Continue.
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

/// **Test: Middleware before returns false stops the chain; handler is not run.**
///
/// **Setup:** One blocking middleware (before returns false), one handler.
/// **Action:** `chain.handle(&message)`.
/// **Expected:** result is Stop; handle_count=0.
#[tokio::test]
async fn test_middleware_stops_chain() {
    struct BlockingMiddleware;

    #[async_trait::async_trait]
    impl Middleware for BlockingMiddleware {
        async fn before(&self, _message: &Message) -> dbot_core::Result<bool> {
            Ok(false)
        }

        async fn after(&self, _message: &Message, _response: &HandlerResponse) -> dbot_core::Result<()> {
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

/// **Test: Handler returns Reply; chain stops and Reply is passed to middleware after.**
///
/// **Setup:** One middleware that in after() asserts Reply content; one handler that returns Reply("AI reply.").
/// **Action:** `chain.handle(&message)`.
/// **Expected:** result is Reply("AI reply."); after_count=1 and middleware sees the reply text.
#[tokio::test]
async fn test_handler_reply_stops_chain_and_passes_to_after() {
    struct ReplyHandler;

    #[async_trait::async_trait]
    impl Handler for ReplyHandler {
        async fn handle(&self, _message: &Message) -> dbot_core::Result<HandlerResponse> {
            Ok(HandlerResponse::Reply("AI reply.".to_string()))
        }
    }

    let after_count = Arc::new(AtomicUsize::new(0));

    struct CaptureResponseMiddleware {
        after_count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Middleware for CaptureResponseMiddleware {
        async fn before(&self, _message: &Message) -> dbot_core::Result<bool> {
            Ok(true)
        }

        async fn after(&self, _message: &Message, response: &HandlerResponse) -> dbot_core::Result<()> {
            self.after_count.fetch_add(1, Ordering::SeqCst);
            if let HandlerResponse::Reply(text) = response {
                assert_eq!(text, "AI reply.");
            }
            Ok(())
        }
    }

    let chain = HandlerChain::new()
        .add_middleware(Arc::new(CaptureResponseMiddleware {
            after_count: after_count.clone(),
        }))
        .add_handler(Arc::new(ReplyHandler));

    let message = create_test_message("test");
    let result = chain.handle(&message).await.unwrap();

    assert_eq!(result, HandlerResponse::Reply("AI reply.".to_string()));
    assert_eq!(after_count.load(Ordering::SeqCst), 1);
}

/// **Test: Multiple middleware run before in order (first, second), after in reverse (second, first).**
///
/// **Setup:** Two middleware that push "before_NAME" and "after_NAME" to a shared vec.
/// **Action:** `chain.handle(&message)` (no handlers so no Reply/Stop from handler).
/// **Expected:** Order is before_first, before_second, after_second, after_first.
#[tokio::test]
async fn test_multiple_middleware_executed_in_order() {
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    struct OrderMiddleware {
        name: String,
        order: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl Middleware for OrderMiddleware {
        async fn before(&self, _message: &Message) -> dbot_core::Result<bool> {
            self.order.lock().unwrap().push(format!("before_{}", self.name));
            Ok(true)
        }

        async fn after(&self, _message: &Message, _response: &HandlerResponse) -> dbot_core::Result<()> {
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

// --- Helpers used by tests ---

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
    async fn before(&self, _message: &Message) -> dbot_core::Result<bool> {
        self.before_count.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }

    async fn after(&self, _message: &Message, _response: &HandlerResponse) -> dbot_core::Result<()> {
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
    async fn handle(&self, _message: &Message) -> dbot_core::Result<HandlerResponse> {
        self.handle_count.fetch_add(1, Ordering::SeqCst);
        Ok(HandlerResponse::Continue)
    }
}
