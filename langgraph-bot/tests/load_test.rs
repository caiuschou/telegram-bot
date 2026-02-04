//! Unit tests for `langgraph_bot::load` module.
//!
//! **BDD style**: Given valid/invalid JSON, when loading, then messages are converted correctly
//! or appropriate errors/stats are returned.

use langgraph::Message;
use langgraph_bot::{
    load_messages_from_slice, load_messages_from_slice_with_stats,
    load_messages_from_slice_with_user_info, load_messages_from_slice_with_user_info_with_stats,
};

/// **Test: Valid JSON with received/sent directions converts to User/Assistant.**
#[test]
fn load_valid_json_converts_received_to_user_and_sent_to_assistant() {
    let json = r#"[
        {"id":"1","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"Hello","direction":"received","created_at":"2025-02-01T10:00:00Z"},
        {"id":"2","user_id":2,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"Hi there","direction":"sent","created_at":"2025-02-01T10:00:15Z"}
    ]"#;
    let (messages, skipped) = load_messages_from_slice_with_stats(json.as_bytes()).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(skipped, 0);
    assert!(matches!(&messages[0], Message::User(s) if s == "Hello"));
    assert!(matches!(&messages[1], Message::Assistant(s) if s == "Hi there"));
}

/// **Test: Messages with non-received/sent direction are skipped and counted.**
#[test]
fn load_skips_unknown_direction_and_returns_count() {
    let json = r#"[
        {"id":"1","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"A","direction":"received","created_at":"2025-02-01T10:00:00Z"},
        {"id":"2","user_id":2,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"B","direction":"unknown","created_at":"2025-02-01T10:00:01Z"},
        {"id":"3","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"C","direction":"sent","created_at":"2025-02-01T10:00:02Z"}
    ]"#;
    let (messages, skipped) = load_messages_from_slice_with_stats(json.as_bytes()).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(skipped, 1);
    assert!(matches!(&messages[0], Message::User(s) if s == "A"));
    assert!(matches!(&messages[1], Message::Assistant(s) if s == "C"));
}

/// **Test: Empty JSON array returns empty messages and zero skipped.**
#[test]
fn load_empty_array_returns_empty() {
    let (messages, skipped) = load_messages_from_slice_with_stats(b"[]").unwrap();
    assert!(messages.is_empty());
    assert_eq!(skipped, 0);
}

/// **Test: Invalid JSON returns error.**
#[test]
fn load_invalid_json_returns_error() {
    let result = load_messages_from_slice(b"not json");
    assert!(result.is_err());
}

/// **Test: seed_messages_to_messages preserves order (via load_messages_from_slice).**
#[test]
fn load_preserves_order() {
    let json = r#"[
        {"id":"1","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"First","direction":"received","created_at":"2025-02-01T10:00:00Z"},
        {"id":"2","user_id":2,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"Second","direction":"sent","created_at":"2025-02-01T10:00:01Z"},
        {"id":"3","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"Third","direction":"received","created_at":"2025-02-01T10:00:02Z"}
    ]"#;
    let messages = load_messages_from_slice(json.as_bytes()).unwrap();
    assert_eq!(messages.len(), 3);
    assert!(matches!(&messages[0], Message::User(s) if s == "First"));
    assert!(matches!(&messages[1], Message::Assistant(s) if s == "Second"));
    assert!(matches!(&messages[2], Message::User(s) if s == "Third"));
}

// ---- With user info (task 1.2 / 1.4) ----

/// **Test: With user info, User message gets prefix; Assistant unchanged; null user fields yield "-".**
#[test]
fn load_with_user_info_prefixes_user_message_and_leaves_assistant_unchanged() {
    let json = r#"[
        {"id":"1","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"Hello","direction":"received","created_at":"2025-02-01T10:00:00Z"},
        {"id":"2","user_id":2,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"Hi there","direction":"sent","created_at":"2025-02-01T10:00:15Z"}
    ]"#;
    let messages = load_messages_from_slice_with_user_info(json.as_bytes()).unwrap();
    assert_eq!(messages.len(), 2);
    assert!(matches!(&messages[0], Message::User(s) if s == "[User: - / -] Hello"));
    assert!(matches!(&messages[1], Message::Assistant(s) if s == "Hi there"));
}

/// **Test: With user info, first_name and username produce expected prefix.**
#[test]
fn load_with_user_info_includes_first_name_and_username_in_prefix() {
    let json = r#"[
        {"id":"1","user_id":1,"chat_id":1,"username":"alice","first_name":"Alice","last_name":"Smith","message_type":"text","content":"Hi","direction":"received","created_at":"2025-02-01T10:00:00Z"}
    ]"#;
    let messages = load_messages_from_slice_with_user_info(json.as_bytes()).unwrap();
    assert_eq!(messages.len(), 1);
    assert!(matches!(&messages[0], Message::User(s) if s == "[User: Alice Smith / @alice] Hi"));
}

/// **Test: With user info, non-received/sent directions are skipped and count returned.**
#[test]
fn load_with_user_info_skips_unknown_direction_and_returns_count() {
    let json = r#"[
        {"id":"1","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"A","direction":"received","created_at":"2025-02-01T10:00:00Z"},
        {"id":"2","user_id":2,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"B","direction":"unknown","created_at":"2025-02-01T10:00:01Z"},
        {"id":"3","user_id":1,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"C","direction":"sent","created_at":"2025-02-01T10:00:02Z"}
    ]"#;
    let (messages, skipped) =
        load_messages_from_slice_with_user_info_with_stats(json.as_bytes()).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(skipped, 1);
    assert!(matches!(&messages[0], Message::User(s) if s == "[User: - / -] A"));
    assert!(matches!(&messages[1], Message::Assistant(s) if s == "C"));
}
