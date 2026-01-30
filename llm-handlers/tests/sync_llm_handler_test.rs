//! Unit tests for SyncLLMHandler.
//!
//! Covers: is_bot_mentioned, extract_question, get_question.
//! Uses in-memory store, MockEmbeddingService, MockBot, and OpenAILlmClient (dummy key); does not call Telegram or OpenAI.

use llm_client::OpenAILlmClient;
use llm_handlers::SyncLLMHandler;
use async_trait::async_trait;
use chrono::Utc;
use dbot_core::{Bot as CoreBot, Chat, Message, MessageDirection, Result as DbotResult, User};
use embedding::EmbeddingService;
use memory::MemoryStore;
use memory_inmemory::InMemoryVectorStore;
use std::sync::Arc;
use storage::MessageRepository;

/// Mock embedding service for tests: returns fixed-dimension vectors, no external API.
struct MockEmbeddingService;

#[async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Ok(vec![0.0; 1536])
    }

    async fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.0; 1536]).collect())
    }
}

/// Mock Bot for tests: no network, returns Ok / dummy id.
struct MockBot;

#[async_trait]
impl CoreBot for MockBot {
    async fn send_message(&self, _chat: &Chat, _text: &str) -> DbotResult<()> {
        Ok(())
    }

    async fn reply_to(&self, _message: &Message, _text: &str) -> DbotResult<()> {
        Ok(())
    }

    async fn edit_message(&self, _chat: &Chat, _message_id: &str, _text: &str) -> DbotResult<()> {
        Ok(())
    }

    async fn send_message_and_return_id(&self, _chat: &Chat, _text: &str) -> DbotResult<String> {
        Ok("1".to_string())
    }
}

/// Builds a minimal SyncLLMHandler for unit testing (repo + in-memory store + mock embedding + MockBot + OpenAILlmClient; no real Telegram/OpenAI).
async fn test_handler(bot_username: Option<&str>) -> SyncLLMHandler {
    let username = Arc::new(tokio::sync::RwLock::new(
        bot_username.map(String::from),
    ));
    let llm_client = Arc::new(OpenAILlmClient::new("dummy_key".to_string()));
    let bot: Arc<dyn CoreBot> = Arc::new(MockBot);
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("in-memory repo");
    let memory_store: Arc<dyn MemoryStore> = Arc::new(InMemoryVectorStore::new());
    let embedding_service: Arc<dyn EmbeddingService> = Arc::new(MockEmbeddingService);

    SyncLLMHandler::new(
        username,
        llm_client,
        bot,
        repo,
        memory_store,
        None,
        embedding_service,
        false,
        "思考中...".to_string(),
        10,
        5,
        0.0,
        5,
    )
}

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

/// **Test: is_bot_mentioned returns true when text contains @my_bot (any position).**
#[tokio::test]
async fn test_is_bot_mentioned_contains_mention() {
    let h = test_handler(Some("my_bot")).await;
    assert!(h.is_bot_mentioned("Hello @my_bot what's the weather?", "my_bot"));
    assert!(h.is_bot_mentioned("@my_bot", "my_bot"));
    assert!(h.is_bot_mentioned("prefix @my_bot suffix", "my_bot"));
}

/// **Test: is_bot_mentioned returns false for no @, @other_bot, or plain "my_bot".**
#[tokio::test]
async fn test_is_bot_mentioned_no_mention() {
    let h = test_handler(Some("my_bot")).await;
    assert!(!h.is_bot_mentioned("Hello world", "my_bot"));
    assert!(!h.is_bot_mentioned("@other_bot", "my_bot"));
    assert!(!h.is_bot_mentioned("my_bot", "my_bot")); // no @
}

// --- extract_question ---

#[tokio::test]
async fn test_extract_question_removes_mention_and_trims() {
    let h = test_handler(Some("bot")).await;
    assert_eq!(
        h.extract_question("  @bot  what is Rust?  ", "bot"),
        "what is Rust?"
    );
    assert_eq!(h.extract_question("@bot hello", "bot"), "hello");
    assert_eq!(h.extract_question("@bot", "bot"), "");
}

/// **Test: When no @-mention, extract_question returns trimmed content.**
#[tokio::test]
async fn test_extract_question_no_mention_returns_trimmed() {
    let h = test_handler(Some("bot")).await;
    assert_eq!(
        h.extract_question("  just a question  ", "bot"),
        "just a question"
    );
}

// --- get_question ---

#[tokio::test]
async fn test_get_question_reply_to_bot_returns_content() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("What is 2+2?", Some("prev_id".to_string()), true);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, Some("What is 2+2?".to_string()));
}

/// **Test: When replying to non-bot message, get_question returns None.**
#[tokio::test]
async fn test_get_question_reply_to_non_bot_returns_none() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("What is 2+2?", Some("user_msg_id".to_string()), false);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, None);
}

#[tokio::test]
async fn test_get_question_mention_with_non_empty_question() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("@bot tell me the time", None, false);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, Some("tell me the time".to_string()));
}

/// **Test: @bot only (no text) returns DEFAULT_EMPTY_MENTION_QUESTION.**
#[tokio::test]
async fn test_get_question_mention_only_returns_default() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("@bot", None, false);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q.as_deref(), Some(SyncLLMHandler::DEFAULT_EMPTY_MENTION_QUESTION));
}

#[tokio::test]
async fn test_get_question_no_reply_no_mention_returns_none() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("random text", None, false);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, None);
}

/// **Test: When bot_username is None, @bot mention is ignored; returns None.**
#[tokio::test]
async fn test_get_question_no_bot_username_mention_ignored() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("@bot hello", None, false);
    let q = h.get_question(&msg, None);
    assert_eq!(q, None);
}

// --- reply_to_message_content 测试 ---

/// 辅助函数：构造带回复内容的 Message
fn make_message_with_reply_content(
    content: &str,
    reply_to_message_id: Option<String>,
    reply_to_message_from_bot: bool,
    reply_to_message_content: Option<String>,
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
        reply_to_message_content,
    }
}

#[tokio::test]
async fn test_reply_to_bot_with_content_returns_question() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message_with_reply_content(
        "继续说",
        Some("bot_msg_123".to_string()),
        true,
        Some("这是机器人之前的回复内容".to_string()),
    );
    // get_question 应该返回用户当前消息内容
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, Some("继续说".to_string()));
}

#[tokio::test]
async fn test_reply_to_bot_content_is_preserved() {
    // 验证 reply_to_message_content 字段被正确设置
    let msg = make_message_with_reply_content(
        "用户追问",
        Some("bot_msg_456".to_string()),
        true,
        Some("机器人之前说的话".to_string()),
    );
    assert_eq!(msg.reply_to_message_content, Some("机器人之前说的话".to_string()));
    assert_eq!(msg.reply_to_message_id, Some("bot_msg_456".to_string()));
    assert!(msg.reply_to_message_from_bot);
}

#[tokio::test]
async fn test_reply_to_non_bot_with_content() {
    let h = test_handler(Some("bot")).await;
    // 回复非机器人消息时，即使有内容也不应触发 LLM
    let msg = make_message_with_reply_content(
        "回复用户消息",
        Some("user_msg_789".to_string()),
        false,
        Some("其他用户的消息".to_string()),
    );
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, None);
}
