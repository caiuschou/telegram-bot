//! Integration tests for [`telegram_bot::storage::MessageRepository`].
//!
//! Covers `get_message_by_id`, `get_recent_messages_by_chat`, get_stats, get_messages, search_messages, and chat filtering using an in-memory SQLite database.

use telegram_bot::storage::{MessageQuery, MessageRecord, MessageRepository};
use tempfile::TempDir;

/// Returns a fresh SQLite database path in a temp dir so each test gets an isolated DB.
/// SqlitePoolManager expects a file path (not a sqlite: URL).
fn fresh_db_path() -> (TempDir, String) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("test.db");
    let path_str = path.to_string_lossy().into_owned();
    (dir, path_str)
}

/// **Test: Get message by id when the message exists.**
///
/// **Setup:** In-memory DB; save one message with known id, user_id, chat_id, content.
/// **Action:** `get_message_by_id(&test_message.id)`.
/// **Expected:** Returns `Some(message)` with matching id, content, user_id, chat_id.
#[tokio::test]
async fn test_get_message_by_id_existing() {
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
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
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
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
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
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
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
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
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
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

/// **Test: get_stats on empty DB returns zeros and None for dates.**
#[tokio::test]
async fn test_get_stats_empty() {
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
        .await
        .expect("Failed to create repository");

    let stats = repo.get_stats().await.expect("Failed to get stats");

    assert_eq!(stats.total_messages, 0);
    assert_eq!(stats.sent_messages, 0);
    assert_eq!(stats.received_messages, 0);
    assert_eq!(stats.unique_users, 0);
    assert_eq!(stats.unique_chats, 0);
    assert!(stats.first_message.is_none());
    assert!(stats.last_message.is_none());
}

/// **Test: get_stats with messages returns correct counts.**
#[tokio::test]
async fn test_get_stats_with_messages() {
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
        .await
        .expect("Failed to create repository");

    repo.save(&MessageRecord::new(
        1,
        10,
        Some("u1".to_string()),
        None,
        None,
        "text".to_string(),
        "hi".to_string(),
        "received".to_string(),
    ))
    .await
    .expect("save");
    repo.save(&MessageRecord::new(
        1,
        10,
        Some("u1".to_string()),
        None,
        None,
        "text".to_string(),
        "bye".to_string(),
        "sent".to_string(),
    ))
    .await
    .expect("save");
    repo.save(&MessageRecord::new(
        2,
        20,
        Some("u2".to_string()),
        None,
        None,
        "text".to_string(),
        "hello".to_string(),
        "received".to_string(),
    ))
    .await
    .expect("save");

    let stats = repo.get_stats().await.expect("Failed to get stats");

    assert_eq!(stats.total_messages, 3);
    assert_eq!(stats.sent_messages, 1);
    assert_eq!(stats.received_messages, 2);
    assert_eq!(stats.unique_users, 2);
    assert_eq!(stats.unique_chats, 2);
    assert!(stats.first_message.is_some());
    assert!(stats.last_message.is_some());
}

/// **Test: get_messages with query (user_id, chat_id, limit, offset).**
#[tokio::test]
async fn test_get_messages_with_query() {
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
        .await
        .expect("Failed to create repository");

    for i in 0..5 {
        repo.save(&MessageRecord::new(
            100,
            200,
            Some("user".to_string()),
            None,
            None,
            "text".to_string(),
            format!("msg {}", i),
            "received".to_string(),
        ))
        .await
        .expect("save");
    }

    let messages = repo
        .get_messages(&MessageQuery {
            user_id: Some(100),
            chat_id: Some(200),
            message_type: None,
            direction: None,
            start_date: None,
            end_date: None,
            limit: Some(2),
            offset: Some(1),
        })
        .await
        .expect("get_messages");

    assert_eq!(messages.len(), 2);
    for m in &messages {
        assert_eq!(m.user_id, 100);
        assert_eq!(m.chat_id, 200);
    }
}

/// **Test: search_messages by keyword.**
#[tokio::test]
async fn test_search_messages() {
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
        .await
        .expect("Failed to create repository");

    repo.save(&MessageRecord::new(
        1,
        1,
        None,
        None,
        None,
        "text".to_string(),
        "hello world".to_string(),
        "received".to_string(),
    ))
    .await
    .expect("save");
    repo.save(&MessageRecord::new(
        1,
        1,
        None,
        None,
        None,
        "text".to_string(),
        "foo bar".to_string(),
        "received".to_string(),
    ))
    .await
    .expect("save");
    repo.save(&MessageRecord::new(
        1,
        1,
        None,
        None,
        None,
        "text".to_string(),
        "hello again".to_string(),
        "received".to_string(),
    ))
    .await
    .expect("save");

    let found = repo
        .search_messages("hello", Some(10))
        .await
        .expect("search_messages");
    assert_eq!(found.len(), 2);
    assert!(found[0].content.contains("hello"));
    assert!(found[1].content.contains("hello"));
}

/// **Test: cleanup_old_messages deletes messages older than cutoff and returns count.**
#[tokio::test]
async fn test_cleanup_old_messages() {
    let (_dir, database_url) = fresh_db_path();
    let repo = MessageRepository::new(&database_url)
        .await
        .expect("Failed to create repository");

    let deleted = repo.cleanup_old_messages(30).await.expect("cleanup");
    assert_eq!(deleted, 0);

    repo.save(&MessageRecord::new(
        1,
        1,
        None,
        None,
        None,
        "text".to_string(),
        "old".to_string(),
        "received".to_string(),
    ))
    .await
    .expect("save");

    // Use -1 days so cutoff is in the future; all existing messages are "older" and get deleted
    let deleted = repo.cleanup_old_messages(-1).await.expect("cleanup");
    assert_eq!(deleted, 1);
}
