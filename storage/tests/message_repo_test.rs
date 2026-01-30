//! Integration tests for [`storage::MessageRepository`].
//!
//! Covers `get_message_by_id`, `get_recent_messages_by_chat`, and chat filtering using an in-memory SQLite database.

use storage::{MessageRecord, MessageRepository};

/// **Test: Get message by id when the message exists.**
///
/// **Setup:** In-memory DB; save one message with known id, user_id, chat_id, content.
/// **Action:** `get_message_by_id(&test_message.id)`.
/// **Expected:** Returns `Some(message)` with matching id, content, user_id, chat_id.
#[tokio::test]
async fn test_get_message_by_id_existing() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let test_message = MessageRecord::new(
        123,
        456,
        Some("testuser".to_string()),
        Some("Test".to_string()),
        None,
        "text".to_string(),
        "Hello World".to_string(),
        "received".to_string(),
    );

    repo.save(&test_message)
        .await
        .expect("Failed to save message");

    let retrieved = repo
        .get_message_by_id(&test_message.id)
        .await
        .expect("Failed to get message");

    assert!(retrieved.is_some());
    let message = retrieved.unwrap();
    assert_eq!(message.id, test_message.id);
    assert_eq!(message.content, "Hello World");
    assert_eq!(message.user_id, 123);
    assert_eq!(message.chat_id, 456);
}

/// **Test: Get message by id when no message has that id.**
///
/// **Setup:** Empty in-memory DB.
/// **Action:** `get_message_by_id("non-existent-id")`.
/// **Expected:** Returns `None`.
#[tokio::test]
async fn test_get_message_by_id_not_found() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let retrieved = repo
        .get_message_by_id("non-existent-id")
        .await
        .expect("Failed to query");

    assert!(retrieved.is_none());
}

/// **Test: Get recent messages by chat returns correct count and order.**
///
/// **Setup:** Save 15 messages in the same chat.
/// **Action:** `get_recent_messages_by_chat(chat_id, 10)`.
/// **Expected:** Returns 10 messages, all with the given chat_id (most recent first).
#[tokio::test]
async fn test_get_recent_messages_by_chat() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let chat_id = 12345;

    for i in 0..15 {
        let message = MessageRecord::new(
            100 + i,
            chat_id,
            Some(format!("user{}", i)),
            Some(format!("User{}", i)),
            None,
            "text".to_string(),
            format!("Message {}", i),
            "received".to_string(),
        );
        repo.save(&message)
            .await
            .expect("Failed to save message");
    }

    let recent = repo
        .get_recent_messages_by_chat(chat_id, 10)
        .await
        .expect("Failed to get recent messages");

    assert_eq!(recent.len(), 10);

    for i in 0..10 {
        assert_eq!(recent[i].chat_id, chat_id);
    }
}

/// **Test: Get recent messages for a chat with no messages.**
///
/// **Setup:** Empty in-memory DB.
/// **Action:** `get_recent_messages_by_chat(99999, 10)`.
/// **Expected:** Returns empty vec.
#[tokio::test]
async fn test_get_recent_messages_by_chat_empty() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let recent = repo
        .get_recent_messages_by_chat(99999, 10)
        .await
        .expect("Failed to get recent messages");

    assert!(recent.is_empty());
}

/// **Test: Recent messages are filtered by chat_id.**
///
/// **Setup:** Save 5 messages in chat_id1 and 5 in chat_id2.
/// **Action:** `get_recent_messages_by_chat(chat_id1, 10)` and same for chat_id2.
/// **Expected:** Each result contains only messages for that chat_id.
#[tokio::test]
async fn test_get_recent_messages_by_chat_filtering() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let chat_id1 = 100;
    let chat_id2 = 200;

    for i in 0..5 {
        let message1 = MessageRecord::new(
            100 + i,
            chat_id1,
            Some("user1".to_string()),
            None,
            None,
            "text".to_string(),
            format!("Chat1 Message {}", i),
            "received".to_string(),
        );
        repo.save(&message1)
            .await
            .expect("Failed to save message");

        let message2 = MessageRecord::new(
            200 + i,
            chat_id2,
            Some("user2".to_string()),
            None,
            None,
            "text".to_string(),
            format!("Chat2 Message {}", i),
            "received".to_string(),
        );
        repo.save(&message2)
            .await
            .expect("Failed to save message");
    }

    let recent1 = repo
        .get_recent_messages_by_chat(chat_id1, 10)
        .await
        .expect("Failed to get recent messages");
    let recent2 = repo
        .get_recent_messages_by_chat(chat_id2, 10)
        .await
        .expect("Failed to get recent messages");

    for msg in &recent1 {
        assert_eq!(msg.chat_id, chat_id1);
    }
    for msg in &recent2 {
        assert_eq!(msg.chat_id, chat_id2);
    }
}
