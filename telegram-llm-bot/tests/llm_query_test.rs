//! Unit tests for [`LLMQuery`](telegram_llm_bot::LLMQuery).
//! BDD style: each test documents scenario and expected outcome.

use telegram_llm_bot::LLMQuery;

/// **Test: LLMQuery stores chat_id, user_id, question, and optional reply_to_message_id.**
#[test]
fn llm_query_creation() {
    let query = LLMQuery {
        chat_id: 123,
        user_id: 456,
        question: "What is the weather?".to_string(),
        reply_to_message_id: Some("msg123".to_string()),
    };

    assert_eq!(query.chat_id, 123);
    assert_eq!(query.user_id, 456);
    assert_eq!(query.question, "What is the weather?");
    assert_eq!(query.reply_to_message_id, Some("msg123".to_string()));
}

/// **Test: LLMQuery can be created without reply_to_message_id (e.g. @mention without reply).**
#[test]
fn llm_query_without_reply_to() {
    let query = LLMQuery {
        chat_id: 123,
        user_id: 456,
        question: "Hello".to_string(),
        reply_to_message_id: None,
    };

    assert!(query.reply_to_message_id.is_none());
}
