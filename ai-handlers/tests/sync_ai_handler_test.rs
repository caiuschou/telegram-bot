//! Unit tests for SyncAIHandler.
//!
//! Covers: is_bot_mentioned, extract_question, get_question.
//! Uses in-memory SQLite, MockEmbeddingService, and dummy Bot/TelegramBotAI; does not call Telegram or OpenAI.

use ai_handlers::SyncAIHandler;
use async_trait::async_trait;
use chrono::Utc;
use dbot_core::{Chat, Message, MessageDirection, User};
use embedding::EmbeddingService;
use memory::MemoryStore;
use memory_inmemory::InMemoryVectorStore;
use openai_client::OpenAIClient;
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::Bot;

/// Mock embedding service for tests: returns fixed-dimension vectors, no external API.
struct MockEmbeddingService;

#[async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, anyhow::Error> {
        Ok(vec![0.0; 1536])
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        Ok(texts.iter().map(|_| vec![0.0; 1536]).collect())
    }
}

/// Builds a minimal SyncAIHandler for unit testing (repo + in-memory store + mock embedding; no real Telegram/OpenAI).
async fn test_handler(bot_username: Option<&str>) -> SyncAIHandler {
    let username = Arc::new(tokio::sync::RwLock::new(
        bot_username.map(String::from),
    ));
    let openai = OpenAIClient::new("dummy_key".to_string());
    let ai_bot = TelegramBotAI::new(
        bot_username.unwrap_or("test_bot").to_string(),
        openai,
    );
    let bot = Bot::new("dummy_telegram_token");
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("in-memory repo");
    let memory_store: Arc<dyn MemoryStore> = Arc::new(InMemoryVectorStore::new());
    let embedding_service: Arc<dyn EmbeddingService> = Arc::new(MockEmbeddingService);

    SyncAIHandler::new(
        username,
        ai_bot,
        bot,
        repo,
        memory_store,
        embedding_service,
        false,
        "思考中...".to_string(),
        10,
        5,
        0.0,
    )
}

fn make_message(
    content: &str,
    reply_to_message_id: Option<String>,
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
    }
}

// --- is_bot_mentioned ---

#[tokio::test]
async fn test_is_bot_mentioned_contains_mention() {
    let h = test_handler(Some("my_bot")).await;
    assert!(h.is_bot_mentioned("Hello @my_bot what's the weather?", "my_bot"));
    assert!(h.is_bot_mentioned("@my_bot", "my_bot"));
    assert!(h.is_bot_mentioned("prefix @my_bot suffix", "my_bot"));
}

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
async fn test_get_question_reply_to_returns_content() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("What is 2+2?", Some("prev_id".to_string()));
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, Some("What is 2+2?".to_string()));
}

#[tokio::test]
async fn test_get_question_mention_with_non_empty_question() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("@bot tell me the time", None);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, Some("tell me the time".to_string()));
}

#[tokio::test]
async fn test_get_question_mention_only_returns_none() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("@bot", None);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, None);
}

#[tokio::test]
async fn test_get_question_no_reply_no_mention_returns_none() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("random text", None);
    let q = h.get_question(&msg, Some("bot"));
    assert_eq!(q, None);
}

#[tokio::test]
async fn test_get_question_no_bot_username_mention_ignored() {
    let h = test_handler(Some("bot")).await;
    let msg = make_message("@bot hello", None);
    let q = h.get_question(&msg, None);
    assert_eq!(q, None);
}
