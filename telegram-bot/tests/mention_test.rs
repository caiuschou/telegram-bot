//! Unit tests for the `mention` module (is_bot_mentioned, extract_question, get_question).
//! BDD style: each test documents scenario and expected outcome.

use chrono::Utc;
use telegram_bot::{
    extract_question, get_question, is_bot_mentioned, Chat, Message, MessageDirection, User,
};

fn make_message(
    content: &str,
    reply_to_message_id: Option<String>,
    reply_to_message_from_bot: bool,
) -> Message {
    Message {
        id: "msg_1".to_string(),
        user: User {
            id: 123,
            username: Some("user".to_string()),
            first_name: Some("User".to_string()),
            last_name: None,
        },
        chat: Chat {
            id: 456,
            chat_type: "private".to_string(),
        },
        content: content.to_string(),
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: Utc::now(),
        reply_to_message_id,
        reply_to_message_from_bot,
        reply_to_message_content: None,
    }
}

// --- is_bot_mentioned ---

/// **Test: is_bot_mentioned returns true when text contains @bot (any position).**
#[test]
fn is_bot_mentioned_contains_mention() {
    assert!(is_bot_mentioned("Hello @my_bot what's the weather?", "my_bot"));
    assert!(is_bot_mentioned("@my_bot", "my_bot"));
    assert!(is_bot_mentioned("prefix @my_bot suffix", "my_bot"));
    assert!(is_bot_mentioned("@mybot hello", "mybot"));
}

/// **Test: is_bot_mentioned returns false for no @, @other_bot, or plain username without @.**
#[test]
fn is_bot_mentioned_no_mention() {
    assert!(!is_bot_mentioned("Hello world", "my_bot"));
    assert!(!is_bot_mentioned("@other_bot", "my_bot"));
    assert!(!is_bot_mentioned("my_bot", "my_bot"));
    assert!(!is_bot_mentioned("Hello world", "mybot"));
    assert!(!is_bot_mentioned("@otherbot hello", "mybot"));
}

// --- extract_question ---

/// **Test: extract_question removes @mention and trims; when only @mention returns empty.**
#[test]
fn extract_question_removes_mention_and_trims() {
    assert_eq!(
        extract_question("  @bot  what is Rust?  ", "bot"),
        "what is Rust?"
    );
    assert_eq!(extract_question("@bot hello", "bot"), "hello");
    assert_eq!(extract_question("@bot", "bot"), "");
    assert_eq!(
        extract_question("@mybot hello world", "mybot"),
        "hello world"
    );
    assert_eq!(
        extract_question("Hello @mybot how are you?", "mybot"),
        "Hello  how are you?"
    );
    assert_eq!(extract_question("@mybot  ", "mybot"), "");
}

/// **Test: When text has no @-mention, extract_question returns trimmed content unchanged.**
#[test]
fn extract_question_no_mention_returns_trimmed() {
    assert_eq!(
        extract_question("  just a question  ", "bot"),
        "just a question"
    );
}

// --- get_question ---

const DEFAULT_EMPTY: &str = "The user only @mentioned you. Please greet them briefly.";

/// **Test: Reply-to-bot returns message content.**
#[test]
fn get_question_reply_to_bot_returns_content() {
    let msg = make_message("What is 2+2?", Some("prev_id".to_string()), true);
    let q = get_question(&msg, Some("bot"), Some(DEFAULT_EMPTY));
    assert_eq!(q, Some("What is 2+2?".to_string()));
}

/// **Test: Reply to non-bot returns None.**
#[test]
fn get_question_reply_to_non_bot_returns_none() {
    let msg = make_message("What is 2+2?", Some("user_msg_id".to_string()), false);
    let q = get_question(&msg, Some("bot"), Some(DEFAULT_EMPTY));
    assert_eq!(q, None);
}

/// **Test: @mention with non-empty text returns extracted question.**
#[test]
fn get_question_mention_with_non_empty_returns_extracted() {
    let msg = make_message("@bot tell me the time", None, false);
    let q = get_question(&msg, Some("bot"), Some(DEFAULT_EMPTY));
    assert_eq!(q, Some("tell me the time".to_string()));
}

/// **Test: @mention only (empty text) with default returns default string.**
#[test]
fn get_question_mention_only_with_default_returns_default() {
    let msg = make_message("@bot", None, false);
    let q = get_question(&msg, Some("bot"), Some(DEFAULT_EMPTY));
    assert_eq!(q, Some(DEFAULT_EMPTY.to_string()));
}

/// **Test: @mention only with empty_mention_default None returns None.**
#[test]
fn get_question_mention_only_without_default_returns_none() {
    let msg = make_message("@bot", None, false);
    let q = get_question(&msg, Some("bot"), None);
    assert_eq!(q, None);
}

/// **Test: No reply and no @mention returns None.**
#[test]
fn get_question_no_reply_no_mention_returns_none() {
    let msg = make_message("random text", None, false);
    let q = get_question(&msg, Some("bot"), Some(DEFAULT_EMPTY));
    assert_eq!(q, None);
}

/// **Test: bot_username None ignores @mention and returns None.**
#[test]
fn get_question_no_bot_username_mention_ignored() {
    let msg = make_message("@bot hello", None, false);
    let q = get_question(&msg, None, Some(DEFAULT_EMPTY));
    assert_eq!(q, None);
}
