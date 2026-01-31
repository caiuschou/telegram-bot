//! Integration tests for [`handler_chain::HandlerChain`].
//!
//! Covers: handler before/after order, handler before stopping the chain, Reply stopping the chain
//! and being passed to handler after, and multiple handlers executed in order (before first→last, after last→first).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::Utc;
use dbot_core::{Chat, Handler, HandlerResponse, Message, MessageDirection, User};
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

/// **Test: Handler before and after run; handle runs once.**
///
/// **Setup:** One handler (counts before/after), one handler (counts handle).
/// **Action:** `chain.handle(&message)`.
/// **Expected:** before_count=1, handle_count=1, after_count=1; response is Continue.
#[tokio::test]
async fn test_handler_chain_with_handler() {
    let before_count = Arc::new(AtomicUsize::new(0));
    let after_count = Arc::new(AtomicUsize::new(0));
    let handle_count = Arc::new(AtomicUsize::new(0));

    let before_after_handler = Arc::new(TestBeforeAfterHandler::new(before_count.clone(), after_count.clone()));
    let handle_only = Arc::new(TestHandler::new(handle_count.clone()));

    let chain = HandlerChain::new()
        .add_handler(before_after_handler)
        .add_handler(handle_only);

    let message = create_test_message("test");
    chain.handle(&message).await.unwrap();

    assert_eq!(before_count.load(Ordering::SeqCst), 1);
    assert_eq!(handle_count.load(Ordering::SeqCst), 1);
    assert_eq!(after_count.load(Ordering::SeqCst), 1);
}

/// **Test: Handler before returns false stops the chain; handle is not run.**
///
/// **Setup:** One blocking handler (before returns false), one handler.
/// **Action:** `chain.handle(&message)`.
/// **Expected:** result is Stop; handle_count=0.
#[tokio::test]
async fn test_handler_stops_chain() {
    struct BlockingHandler;

    #[async_trait::async_trait]
    impl Handler for BlockingHandler {
        async fn before(&self, _message: &Message) -> dbot_core::Result<bool> {
            Ok(false)
        }
    }

    let handle_count = Arc::new(AtomicUsize::new(0));
    let handler = Arc::new(TestHandler::new(handle_count.clone()));

    let chain = HandlerChain::new()
        .add_handler(Arc::new(BlockingHandler))
        .add_handler(handler);

    let message = create_test_message("test");
    let result = chain.handle(&message).await.unwrap();

    assert_eq!(result, HandlerResponse::Stop);
    assert_eq!(handle_count.load(Ordering::SeqCst), 0);
}

/// **Test: Handler returns Reply; chain stops and Reply is passed to handler after.**
///
/// **Setup:** One handler that in after() asserts Reply content; one handler that returns Reply("AI reply.").
/// **Action:** `chain.handle(&message)`.
/// **Expected:** result is Reply("AI reply."); after_count=1 and handler sees the reply text.
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

    struct CaptureResponseHandler {
        after_count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Handler for CaptureResponseHandler {
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
        .add_handler(Arc::new(CaptureResponseHandler {
            after_count: after_count.clone(),
        }))
        .add_handler(Arc::new(ReplyHandler));

    let message = create_test_message("test");
    let result = chain.handle(&message).await.unwrap();

    assert_eq!(result, HandlerResponse::Reply("AI reply.".to_string()));
    assert_eq!(after_count.load(Ordering::SeqCst), 1);
}

/// **Test: Multiple handlers run before in order (first, second), after in reverse (second, first).**
///
/// **Setup:** Two handlers that push "before_NAME" and "after_NAME" to a shared vec.
/// **Action:** `chain.handle(&message)` (handle phase returns Continue).
/// **Expected:** Order is before_first, before_second, after_second, after_first.
#[tokio::test]
async fn test_multiple_handlers_executed_in_order() {
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    struct OrderHandler {
        name: String,
        order: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl Handler for OrderHandler {
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
        .add_handler(Arc::new(OrderHandler {
            name: "first".to_string(),
            order: order.clone(),
        }))
        .add_handler(Arc::new(OrderHandler {
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

struct TestBeforeAfterHandler {
    before_count: Arc<AtomicUsize>,
    after_count: Arc<AtomicUsize>,
}

impl TestBeforeAfterHandler {
    fn new(before_count: Arc<AtomicUsize>, after_count: Arc<AtomicUsize>) -> Self {
        Self { before_count, after_count }
    }
}

#[async_trait::async_trait]
impl Handler for TestBeforeAfterHandler {
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
