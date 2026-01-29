//! Unit tests for `Context` and `ContextMetadata`.
//!
//! Tests context formatting for AI model input and token limit checks.
//! External interactions: AI model APIs (format_for_model output), token limits.

use super::*;
use chrono::Utc;

fn make_context(
    system_message: Option<String>,
    conversation_history: Vec<String>,
    user_preferences: Option<String>,
    total_tokens: usize,
) -> Context {
    Context {
        system_message,
        conversation_history,
        user_preferences,
        metadata: ContextMetadata {
            user_id: Some("user1".to_string()),
            conversation_id: Some("conv1".to_string()),
            total_tokens,
            message_count: 2,
            created_at: Utc::now(),
        },
    }
}

#[test]
fn test_format_for_model_with_system() {
    let ctx = make_context(
        Some("You are helpful.".to_string()),
        vec!["User: Hi".to_string(), "Assistant: Hello".to_string()],
        None,
        10,
    );
    let out = ctx.format_for_model(true);
    assert!(out.contains("System: You are helpful."));
    assert!(out.contains("User: Hi"));
    assert!(out.contains("Assistant: Hello"));
}

#[test]
fn test_format_for_model_without_system() {
    let ctx = make_context(
        Some("You are helpful.".to_string()),
        vec!["User: Hi".to_string()],
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
        Some("Pref: English".to_string()),
        5,
    );
    let out = ctx.format_for_model(true);
    assert!(out.contains("User Preferences: Pref: English"));
    assert!(out.contains("User: Hi"));
}

#[test]
fn test_exceeds_limit_true() {
    let ctx = make_context(None, vec![], None, 100);
    assert!(ctx.exceeds_limit(50));
}

#[test]
fn test_exceeds_limit_false() {
    let ctx = make_context(None, vec![], None, 30);
    assert!(!ctx.exceeds_limit(50));
}

#[test]
fn test_exceeds_limit_equal() {
    let ctx = make_context(None, vec![], None, 50);
    assert!(!ctx.exceeds_limit(50));
}
