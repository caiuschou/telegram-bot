//! Unit tests for `prompt::format_for_model`.
//!
//! Verifies system message, preferences, and recent/semantic section formatting.
//! External interactions: none (pure function tests).

use prompt::{
    format_for_model, format_for_model_as_messages, format_for_model_as_messages_with_roles,
    parse_message_line, ChatMessage, MessageRole, SECTION_RECENT, SECTION_SEMANTIC,
};

/// **Test: When include_system is true and system_message is set, output contains "System: {message}".**
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

/// **Test: When include_system is false, output does not contain "System:".**
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

/// **Test: When include_system is true but system_message is None, output has no System line.**
#[test]
fn format_include_system_true_none_system_message() {
    let out = format_for_model(true, None, None, &[] as &[&str], &[] as &[&str]);
    assert!(!out.contains("System:"));
    assert!(out.is_empty());
}

/// **Test: When user_preferences is set, output contains "User Preferences: {preferences}".**
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

/// **Test: Recent and semantic sections are present with correct content (SECTION_RECENT, SECTION_SEMANTIC, lines).**
#[test]
fn format_includes_recent_and_semantic_sections() {
    let out = format_for_model(
        false,
        None,
        None,
        &["User: Hi", "Assistant: Hello"],
        &["User: cat", "Assistant: Cats eat fish."],
    );
    assert!(out.contains(SECTION_RECENT));
    assert!(out.contains("User: Hi"));
    assert!(out.contains("Assistant: Hello"));
    assert!(out.contains(SECTION_SEMANTIC));
    assert!(out.contains("cat"));
    assert!(out.contains("Cats eat fish."));
}

/// **Test: format_for_model accepts Vec<String> and Vec<&str> for recent and semantic iterators.**
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

/// **Test: With no system/preferences/recent/semantic, result is a single User message with current_question.**
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

/// **Test: When include_system and system_message are set, first message is System, then User(question).**
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

/// **Test: One User message contains preferences + recent block; second message is current question.**
#[test]
fn format_as_messages_context_block_then_question() {
    let msgs = format_for_model_as_messages(
        false,
        None,
        Some("Pref: tea"),
        &["User: What do dogs eat?", "Assistant: Dogs eat dog food."],
        &[] as &[&str],
        "What about cats?",
    );
    assert_eq!(msgs.len(), 2);
    assert!(matches!(msgs[0].role, MessageRole::User));
    assert!(msgs[0].content.contains(SECTION_RECENT));
    assert!(msgs[0].content.contains("User Preferences: Pref: tea"));
    assert!(msgs[0].content.contains("What do dogs eat?"));
    assert!(matches!(msgs[1].role, MessageRole::User));
    assert_eq!(msgs[1].content, "What about cats?");
}

/// **Test: ChatMessage::system/user/assistant set role and content correctly.**
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

/// **Test: "User: hello" parses to User role with content "hello".**
#[test]
fn parse_message_line_user() {
    let msg = parse_message_line("User: hello").unwrap();
    assert!(matches!(msg.role, MessageRole::User));
    assert_eq!(msg.content, "hello");
}

/// **Test: "Assistant: hi there" parses to Assistant role with trimmed content.**
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

/// **Test: Content after prefix is trimmed (e.g. "  User:  foo  " -> content "foo").**
#[test]
fn parse_message_line_trimmed() {
    let msg = parse_message_line("  User:  foo  ").unwrap();
    assert_eq!(msg.content, "foo");
}

/// **Test: Empty line or unknown prefix returns None.**
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
        &["User: What do dogs eat?", "Assistant: Dogs eat dog food."],
        &[] as &[&str],
        "What about cats?",
    );
    assert_eq!(msgs.len(), 2, "recent as one User block, then current question");
    assert!(matches!(msgs[0].role, MessageRole::User));
    assert!(msgs[0].content.contains("Conversation (recent):"));
    assert!(msgs[0].content.contains("What do dogs eat?"));
    assert!(msgs[0].content.contains("Dogs eat dog food."));
    assert!(matches!(msgs[1].role, MessageRole::User));
    assert_eq!(msgs[1].content, "What about cats?");
}

/// **Test: Context block can be semantic-only (no recent, no preferences).**
#[test]
fn format_with_roles_semantic_only_then_question() {
    let msgs = format_for_model_as_messages_with_roles(
        false,
        None,
        None,
        &[] as &[&str],
        &["Ref: cats are furry."],
        "Tell me more.",
    );
    assert_eq!(msgs.len(), 2);
    assert!(msgs[0].content.contains(SECTION_SEMANTIC));
    assert!(msgs[0].content.contains("Ref: cats are furry."));
    assert_eq!(msgs[1].content, "Tell me more.");
}

/// **Test: Order is System, User(recent block), User(question); three messages.**
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
