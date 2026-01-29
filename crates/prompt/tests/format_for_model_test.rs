//! Unit tests for `prompt::format_for_model`.
//!
//! Verifies system message, preferences, and recent/semantic section formatting.
//! External interactions: none (pure function tests).

use prompt::{
    format_for_model, format_for_model_as_messages, format_for_model_as_messages_with_roles,
    parse_message_line, ChatMessage, MessageRole, SECTION_RECENT, SECTION_SEMANTIC,
};

#[test]
fn format_includes_system_when_requested() {
    let out = format_for_model(
        true,
        Some("You are helpful."),
        None,
        &[] as &[&str],
        &[] as &[&str],
    );
    assert!(out.contains("System: You are helpful."));
}

#[test]
fn format_omits_system_when_not_requested() {
    let out = format_for_model(
        false,
        Some("You are helpful."),
        None,
        &[] as &[&str],
        &[] as &[&str],
    );
    assert!(!out.contains("System:"));
}

#[test]
fn format_includes_preferences() {
    let out = format_for_model(
        false,
        None,
        Some("Pref: English"),
        &[] as &[&str],
        &[] as &[&str],
    );
    assert!(out.contains("User Preferences: Pref: English"));
}

#[test]
fn format_includes_recent_and_semantic_sections() {
    let out = format_for_model(
        false,
        None,
        None,
        &["User: Hi", "Assistant: Hello"],
        &["User: 猫", "Assistant: 猫吃鱼"],
    );
    assert!(out.contains(SECTION_RECENT));
    assert!(out.contains("User: Hi"));
    assert!(out.contains("Assistant: Hello"));
    assert!(out.contains(SECTION_SEMANTIC));
    assert!(out.contains("猫"));
    assert!(out.contains("猫吃鱼"));
}

#[test]
fn format_accepts_string_slices() {
    let recent: Vec<String> = vec!["A".into(), "B".into()];
    let semantic: Vec<&str> = vec!["C", "D"];
    let out = format_for_model(
        false,
        None,
        None,
        &recent,
        &semantic,
    );
    assert!(out.contains("A"));
    assert!(out.contains("B"));
    assert!(out.contains("C"));
    assert!(out.contains("D"));
}

// --- format_for_model_as_messages (OpenAI one-to-one) ---

#[test]
fn format_as_messages_returns_user_question_only_when_no_context() {
    let msgs = format_for_model_as_messages(
        false,
        None,
        None,
        &[] as &[&str],
        &[] as &[&str],
        "What is AI?",
    );
    assert_eq!(msgs.len(), 1);
    assert!(matches!(msgs[0].role, MessageRole::User));
    assert_eq!(msgs[0].content, "What is AI?");
}

#[test]
fn format_as_messages_includes_system_when_requested() {
    let msgs = format_for_model_as_messages(
        true,
        Some("You are helpful."),
        None,
        &[] as &[&str],
        &[] as &[&str],
        "Hi",
    );
    assert!(msgs.len() >= 2);
    assert!(matches!(msgs[0].role, MessageRole::System));
    assert_eq!(msgs[0].content, "You are helpful.");
    assert!(matches!(msgs[1].role, MessageRole::User));
    assert_eq!(msgs[1].content, "Hi");
}

#[test]
fn format_as_messages_context_block_then_question() {
    let msgs = format_for_model_as_messages(
        false,
        None,
        Some("Pref: tea"),
        &["User: 狗吃什么", "Assistant: 狗吃狗粮。"],
        &[] as &[&str],
        "那猫呢？",
    );
    assert_eq!(msgs.len(), 2);
    assert!(matches!(msgs[0].role, MessageRole::User));
    assert!(msgs[0].content.contains(SECTION_RECENT));
    assert!(msgs[0].content.contains("User Preferences: Pref: tea"));
    assert!(msgs[0].content.contains("狗吃什么"));
    assert!(matches!(msgs[1].role, MessageRole::User));
    assert_eq!(msgs[1].content, "那猫呢？");
}

#[test]
fn chat_message_constructors() {
    let s = ChatMessage::system("sys");
    assert!(matches!(s.role, MessageRole::System));
    assert_eq!(s.content, "sys");
    let u = ChatMessage::user("usr");
    assert!(matches!(u.role, MessageRole::User));
    assert_eq!(u.content, "usr");
    let a = ChatMessage::assistant("ast");
    assert!(matches!(a.role, MessageRole::Assistant));
    assert_eq!(a.content, "ast");
}

// --- parse_message_line (User / Assistant / System) ---

#[test]
fn parse_message_line_user() {
    let msg = parse_message_line("User: hello").unwrap();
    assert!(matches!(msg.role, MessageRole::User));
    assert_eq!(msg.content, "hello");
}

#[test]
fn parse_message_line_assistant() {
    let msg = parse_message_line("Assistant: hi there").unwrap();
    assert!(matches!(msg.role, MessageRole::Assistant));
    assert_eq!(msg.content, "hi there");
}

#[test]
fn parse_message_line_system() {
    let msg = parse_message_line("System: You are helpful.").unwrap();
    assert!(matches!(msg.role, MessageRole::System));
    assert_eq!(msg.content, "You are helpful.");
}

#[test]
fn parse_message_line_trimmed() {
    let msg = parse_message_line("  User:  foo  ").unwrap();
    assert_eq!(msg.content, "foo");
}

#[test]
fn parse_message_line_unknown_returns_none() {
    assert!(parse_message_line("").is_none());
    assert!(parse_message_line("NoPrefix: x").is_none());
}

// --- format_for_model_as_messages_with_roles (context returns different types) ---

#[test]
fn format_with_roles_recent_as_one_user_block() {
    let msgs = format_for_model_as_messages_with_roles(
        false,
        None,
        None,
        &["User: 狗吃什么", "Assistant: 狗吃狗粮。"],
        &[] as &[&str],
        "那猫呢？",
    );
    assert_eq!(msgs.len(), 2, "recent as one User block, then current question");
    assert!(matches!(msgs[0].role, MessageRole::User));
    assert!(msgs[0].content.contains("Conversation (recent):"));
    assert!(msgs[0].content.contains("狗吃什么"));
    assert!(msgs[0].content.contains("狗吃狗粮。"));
    assert!(matches!(msgs[1].role, MessageRole::User));
    assert_eq!(msgs[1].content, "那猫呢？");
}

#[test]
fn format_with_roles_includes_system_then_recent_then_question() {
    let msgs = format_for_model_as_messages_with_roles(
        true,
        Some("You are helpful."),
        None,
        &["User: Hi", "Assistant: Hello"],
        &[] as &[&str],
        "Bye",
    );
    assert_eq!(msgs.len(), 3, "system, one User(recent block), User(question)");
    assert!(matches!(msgs[0].role, MessageRole::System));
    assert_eq!(msgs[0].content, "You are helpful.");
    assert!(matches!(msgs[1].role, MessageRole::User));
    assert!(msgs[1].content.contains("Conversation (recent):"));
    assert!(msgs[1].content.contains("Hi"));
    assert!(msgs[1].content.contains("Hello"));
    assert!(matches!(msgs[2].role, MessageRole::User));
    assert_eq!(msgs[2].content, "Bye");
}
