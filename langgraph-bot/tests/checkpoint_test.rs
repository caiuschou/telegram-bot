//! Unit tests for `langgraph_bot::checkpoint` verify functions.
//!
//! **BDD style**: Given original and read-back messages, when verifying integrity/format,
//! then correct Ok or Err is returned.

use langgraph::Message;
use langgraph_bot::{verify_messages_format, verify_messages_integrity};

/// **Test: Same length and content passes integrity check.**
#[test]
fn verify_integrity_same_messages_passes() {
    let original = vec![
        Message::User("a".into()),
        Message::Assistant("b".into()),
        Message::User("c".into()),
    ];
    let read_back = vec![
        Message::User("a".into()),
        Message::Assistant("b".into()),
        Message::User("c".into()),
    ];
    assert!(verify_messages_integrity(&original, &read_back).is_ok());
}

/// **Test: Different length fails integrity check.**
#[test]
fn verify_integrity_different_length_fails() {
    let original = vec![Message::User("a".into())];
    let read_back = vec![Message::User("a".into()), Message::Assistant("b".into())];
    let err = verify_messages_integrity(&original, &read_back).unwrap_err();
    assert!(err.to_string().contains("length mismatch"));
}

/// **Test: Same length but different content fails integrity check.**
#[test]
fn verify_integrity_different_content_fails() {
    let original = vec![Message::User("a".into())];
    let read_back = vec![Message::User("b".into())];
    let err = verify_messages_integrity(&original, &read_back).unwrap_err();
    assert!(err.to_string().contains("mismatch"));
}

/// **Test: Same length but different variant fails integrity check.**
#[test]
fn verify_integrity_different_variant_fails() {
    let original = vec![Message::User("a".into())];
    let read_back = vec![Message::Assistant("a".into())];
    let err = verify_messages_integrity(&original, &read_back).unwrap_err();
    assert!(err.to_string().contains("mismatch"));
}

/// **Test: Empty slices pass integrity check.**
#[test]
fn verify_integrity_empty_passes() {
    let original: Vec<Message> = vec![];
    let read_back: Vec<Message> = vec![];
    assert!(verify_messages_integrity(&original, &read_back).is_ok());
}

/// **Test: User and Assistant only pass format check.**
#[test]
fn verify_format_user_assistant_passes() {
    let messages = vec![
        Message::User("a".into()),
        Message::Assistant("b".into()),
    ];
    assert!(verify_messages_format(&messages).is_ok());
}

/// **Test: System message fails format check.**
#[test]
fn verify_format_system_fails() {
    let messages = vec![
        Message::User("a".into()),
        Message::System("sys".into()),
    ];
    let err = verify_messages_format(&messages).unwrap_err();
    assert!(err.to_string().contains("System"));
}

/// **Test: Empty slice passes format check.**
#[test]
fn verify_format_empty_passes() {
    let messages: Vec<Message> = vec![];
    assert!(verify_messages_format(&messages).is_ok());
}
