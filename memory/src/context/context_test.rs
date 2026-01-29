//! Unit tests for `Context` and `ContextMetadata`.
//!
//! Tests context formatting for AI model input and token limit checks.
//! External interactions: AI model APIs (format_for_model output), token limits.

use super::*;
use chrono::Utc;

fn make_context(
    system_message: Option<String>,
    recent_messages: Vec<String>,
    semantic_messages: Vec<String>,
    user_preferences: Option<String>,
    total_tokens: usize,
) -> Context {
    let message_count = recent_messages.len() + semantic_messages.len();
    Context {
        system_message,
        recent_messages,
        semantic_messages,
        user_preferences,
        metadata: ContextMetadata {
            user_id: Some("user1".to_string()),
            conversation_id: Some("conv1".to_string()),
            total_tokens,
            message_count,
            created_at: Utc::now(),
        },
    }
}

#[test]
fn test_format_for_model_with_system() {
    let ctx = make_context(
        Some("You are helpful.".to_string()),
        vec!["User: Hi".to_string(), "Assistant: Hello".to_string()],
        vec![],
        None,
        10,
    );
    let out = ctx.format_for_model(true);
    assert!(out.contains("System: You are helpful."));
    assert!(out.contains("Conversation (recent):"));
    assert!(out.contains("User: Hi"));
    assert!(out.contains("Assistant: Hello"));
}

#[test]
fn test_format_for_model_without_system() {
    let ctx = make_context(
        Some("You are helpful.".to_string()),
        vec!["User: Hi".to_string()],
        vec![],
        None,
        5,
    );
    let out = ctx.format_for_model(false);
    assert!(!out.contains("System:"));
    assert!(out.contains("User: Hi"));
}

#[test]
fn test_format_for_model_with_preferences() {
    let ctx = make_context(
        None,
        vec!["User: Hi".to_string()],
        vec![],
        Some("Pref: English".to_string()),
        5,
    );
    let out = ctx.format_for_model(true);
    assert!(out.contains("User Preferences: Pref: English"));
    assert!(out.contains("User: Hi"));
}

#[test]
fn test_exceeds_limit_true() {
    let ctx = make_context(None, vec![], vec![], None, 100);
    assert!(ctx.exceeds_limit(50));
}

#[test]
fn test_exceeds_limit_false() {
    let ctx = make_context(None, vec![], vec![], None, 30);
    assert!(!ctx.exceeds_limit(50));
}

#[test]
fn test_exceeds_limit_equal() {
    let ctx = make_context(None, vec![], vec![], None, 50);
    assert!(!ctx.exceeds_limit(50));
}

#[test]
fn test_format_for_model_distinguishes_recent_and_semantic() {
    let ctx = make_context(
        None,
        vec!["User: Hi".to_string(), "Assistant: Hello".to_string()],
        vec!["User: 猫吃什么".to_string(), "Assistant: 猫吃鱼".to_string()],
        None,
        20,
    );
    let out = ctx.format_for_model(false);
    assert!(out.contains("Conversation (recent):"));
    assert!(out.contains("Relevant reference (semantic):"));
    assert!(out.contains("User: Hi"));
    assert!(out.contains("Assistant: Hello"));
    assert!(out.contains("猫吃什么"));
    assert!(out.contains("猫吃鱼"));
}

#[test]
fn test_conversation_history_returns_recent_then_semantic() {
    let ctx = make_context(
        None,
        vec!["A".to_string(), "B".to_string()],
        vec!["C".to_string()],
        None,
        5,
    );
    let all = ctx.conversation_history();
    assert_eq!(all, vec!["A", "B", "C"]);
}

#[test]
fn test_is_empty() {
    let empty = make_context(None, vec![], vec![], None, 0);
    assert!(empty.is_empty());
    let with_recent = make_context(None, vec!["x".to_string()], vec![], None, 5);
    assert!(!with_recent.is_empty());
    let with_semantic = make_context(None, vec![], vec!["y".to_string()], None, 5);
    assert!(!with_semantic.is_empty());
}

#[test]
fn test_to_messages_returns_chat_messages_with_different_roles() {
    use prompt::{MessageRole, SECTION_RECENT};

    let ctx = make_context(
        Some("You are helpful.".to_string()),
        vec![
            "User: Hi".to_string(),
            "Assistant: Hello".to_string(),
            "User: 猫吃什么".to_string(),
        ],
        vec!["User: 狗吃什么".to_string(), "Assistant: 狗吃狗粮。".to_string()],
        Some("Pref: tea".to_string()),
        50,
    );
    let msgs = ctx.to_messages(true, "那猫呢？");
    // System, one User(context: preferences + recent + semantic), User(question)
    assert!(msgs.len() >= 3);
    let first = &msgs[0];
    assert!(matches!(first.role, MessageRole::System));
    assert_eq!(first.content, "You are helpful.");
    assert!(matches!(msgs[1].role, MessageRole::User));
    assert!(msgs[1].content.contains(SECTION_RECENT));
    assert!(msgs[1].content.contains("Hi"));
    assert!(msgs[1].content.contains("Hello"));
    assert!(msgs[1].content.contains("猫吃什么"));
    assert!(msgs[1].content.contains("User Preferences: Pref: tea"));
    assert!(msgs[1].content.contains("狗吃什么"));
    assert!(msgs[1].content.contains("狗吃狗粮。"));
    let last = msgs.last().unwrap();
    assert!(matches!(last.role, MessageRole::User));
    assert_eq!(last.content, "那猫呢？");
}
