//! Unit tests for [`LLMDetectionHandler`](telegram_llm_bot::LLMDetectionHandler) `Handler::handle` behaviour.
//! BDD style: each test documents scenario and expected outcome.
//! Mention detection logic is covered by `telegram_bot::mention` tests.

use std::sync::Arc;
use chrono::Utc;
use telegram_bot::{Chat, Handler, HandlerResponse, Message, MessageDirection, User};
use telegram_llm_bot::LLMDetectionHandler;

fn make_handler(bot_username: Option<&str>) -> (LLMDetectionHandler, tokio::sync::mpsc::UnboundedReceiver<telegram_llm_bot::LLMQuery>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handler = LLMDetectionHandler::new(
        Arc::new(tokio::sync::RwLock::new(bot_username.map(String::from))),
        Arc::new(tx),
    );
    (handler, rx)
}

fn make_message(
    content: &str,
    reply_to_message_id: Option<String>,
    reply_to_message_from_bot: bool,
    reply_to_message_content: Option<String>,
) -> Message {
    Message {
        id: "msg123".to_string(),
        user: User {
            id: 456,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: None,
        },
        chat: Chat {
            id: 123,
            chat_type: "group".to_string(),
        },
        content: content.to_string(),
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: Utc::now(),
        reply_to_message_id,
        reply_to_message_from_bot,
        reply_to_message_content,
    }
}

// --- Handler::handle: @mention triggers ---

/// **Test: Message with @bot and non-empty question returns Stop and sends LLMQuery with extracted question.**
#[tokio::test]
async fn handler_with_bot_mention_sends_query() {
    let (handler, mut rx) = make_handler(Some("mybot"));
    let message = make_message("@mybot hello world", None, false, None);

    let result = handler.handle(&message).await;
    assert!(matches!(result, Ok(HandlerResponse::Stop)));

    let received = rx.recv().await.unwrap();
    assert_eq!(received.chat_id, 123);
    assert_eq!(received.user_id, 456);
    assert_eq!(received.question, "hello world");
    assert_eq!(received.reply_to_message_id, None);
}

/// **Test: Message without @mention and not reply-to-bot returns Continue.**
#[tokio::test]
async fn handler_without_bot_mention_returns_continue() {
    let (handler, _rx) = make_handler(Some("mybot"));
    let message = make_message("Hello world", None, false, None);

    let result = handler.handle(&message).await;
    assert!(matches!(result, Ok(HandlerResponse::Continue)));
}

/// **Test: When bot_username is None, @mention does not trigger; returns Continue.**
#[tokio::test]
async fn handler_with_empty_bot_username_ignores_mention() {
    let (handler, _rx) = make_handler(None);
    let message = make_message("@bot hello", None, false, None);

    let result = handler.handle(&message).await;
    assert!(matches!(result, Ok(HandlerResponse::Continue)));
}

// --- Handler::handle: reply-to-bot ---

/// **Test: Reply to bot message returns Stop and sends LLMQuery with full content.**
#[tokio::test]
async fn handler_reply_to_bot_sends_query() {
    let (handler, mut rx) = make_handler(None);
    let message = make_message(
        "This is a reply to bot",
        Some("bot_msg_456".to_string()),
        true,
        Some("Previous bot message".to_string()),
    );

    let result = handler.handle(&message).await;
    assert!(matches!(result, Ok(HandlerResponse::Stop)));

    let received = rx.recv().await.unwrap();
    assert_eq!(received.chat_id, 123);
    assert_eq!(received.user_id, 456);
    assert_eq!(received.question, "This is a reply to bot");
    assert_eq!(received.reply_to_message_id, Some("bot_msg_456".to_string()));
}

/// **Test: Reply to non-bot message does not trigger; returns Continue.**
#[tokio::test]
async fn handler_reply_to_non_bot_does_not_trigger() {
    let (handler, _rx) = make_handler(Some("mybot"));
    let message = make_message(
        "reply to user message",
        Some("user_msg_789".to_string()),
        false,
        Some("User's previous message".to_string()),
    );

    let result = handler.handle(&message).await;
    assert!(matches!(result, Ok(HandlerResponse::Continue)));
}

/// **Test: When both reply-to-bot and @mention present, reply-to-bot wins; question is raw content.**
#[tokio::test]
async fn handler_reply_to_bot_takes_priority_over_mention() {
    let (handler, mut rx) = make_handler(Some("mybot"));
    let message = make_message(
        "@mybot hello world",
        Some("bot_msg_789".to_string()),
        true,
        Some("Previous bot message".to_string()),
    );

    let result = handler.handle(&message).await;
    assert!(matches!(result, Ok(HandlerResponse::Stop)));

    let received = rx.recv().await.unwrap();
    assert_eq!(received.question, "@mybot hello world");
    assert_eq!(received.reply_to_message_id, Some("bot_msg_789".to_string()));
}
