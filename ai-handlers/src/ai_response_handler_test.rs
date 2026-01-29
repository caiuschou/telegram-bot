//! 单元测试：`AIQueryHandler` 的上下文构建、问题格式化、记忆写入与记忆上下文。
//!
//! 依赖：内存 SQLite、InMemoryVectorStore、MockEmbeddingService；不调用真实 Telegram 或 OpenAI。

use crate::ai_mention_detector::AIQuery;
use crate::ai_response_handler::AIQueryHandler;
use async_trait::async_trait;
use embedding::EmbeddingService;
use memory::MemoryRole;
use memory_inmemory::InMemoryVectorStore;
use openai_client::OpenAIClient;
use std::sync::Arc;
use storage::MessageRecord;
use storage::MessageRepository;

/// 测试用 Mock Embedding：返回固定 1536 维向量，不发起任何外部 API 调用。
/// 用于需要 EmbeddingService 但不关心向量内容的测试（如 build_memory_context）。
struct MockEmbeddingService;

#[async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(&self, _text: &str) -> std::result::Result<Vec<f32>, anyhow::Error> {
        Ok(vec![0.0; 1536])
    }

    async fn embed_batch(
        &self,
        texts: &[String],
    ) -> std::result::Result<Vec<Vec<f32>>, anyhow::Error> {
        Ok(texts.iter().map(|_| vec![0.0; 1536]).collect())
    }
}

/// 构造用于测试的 AIQueryHandler：内存 SQLite、InMemory 记忆、Mock Embedding、假 Bot/OpenAI。
/// 与外部无真实网络或 API 调用。
fn new_test_handler(
    repo: MessageRepository,
    memory_store: Arc<dyn memory::MemoryStore>,
    rx: tokio::sync::mpsc::UnboundedReceiver<AIQuery>,
) -> AIQueryHandler {
    let openai_client = OpenAIClient::new("test_key".to_string());
    let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
    AIQueryHandler::new(
        ai_bot,
        teloxide::Bot::new("test_token"),
        repo,
        memory_store,
        Arc::new(MockEmbeddingService),
        rx,
        false,
        "Thinking...".to_string(),
    )
}

/// 测试 build_context：当 query 带有 reply_to_message_id 时，应包含「回复消息」块，
/// 且内容来自 MessageRepository 中该 id 对应的消息（用户名、正文）。
#[tokio::test]
async fn test_build_context_with_reply_to() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let replied_message = MessageRecord::new(
        123,
        456,
        Some("original_user".to_string()),
        Some("Original".to_string()),
        None,
        "text".to_string(),
        "This is the original message".to_string(),
        "received".to_string(),
    );

    repo.save(&replied_message)
        .await
        .expect("Failed to save replied message");

    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = new_test_handler(repo.clone(), memory_store, _rx);

    let query = AIQuery {
        chat_id: 456,
        user_id: 123,
        question: "Can you explain more?".to_string(),
        reply_to_message_id: Some(replied_message.id.clone()),
    };

    let context: String = handler.build_context(&query).await;

    assert!(context.contains("[回复消息]"));
    assert!(context.contains("original_user"));
    assert!(context.contains("This is the original message"));
}

/// 测试 build_context：当该 chat 存在最近消息时，应包含「最近的消息」块，
/// 且按 repo 中该 chat 的最近 10 条逆序展示（含用户名与内容）。
#[tokio::test]
async fn test_build_context_with_recent_messages() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let chat_id = 789;

    for i in 0..5 {
        let message = MessageRecord::new(
            100 + i,
            chat_id,
            Some(format!("user{}", i)),
            Some(format!("User{}", i)),
            None,
            "text".to_string(),
            format!("Message {}", i),
            "received".to_string(),
        );
        repo.save(&message)
            .await
            .expect("Failed to save message");
    }

    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = new_test_handler(repo, memory_store, rx);

    let query = AIQuery {
        chat_id,
        user_id: 100,
        question: "What was discussed?".to_string(),
        reply_to_message_id: None,
    };

    let context: String = handler.build_context(&query).await;

    assert!(context.contains("[最近的消息]"));
    assert!(context.contains("user0"));
    assert!(context.contains("Message 0"));
    assert!(context.contains("Message 4"));
}

/// 测试 build_context：无回复目标且该 chat 无最近消息时，应返回空字符串。
#[tokio::test]
async fn test_build_context_empty() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = new_test_handler(repo, memory_store, rx);

    let query = AIQuery {
        chat_id: 999,
        user_id: 123,
        question: "Hello".to_string(),
        reply_to_message_id: None,
    };

    let context: String = handler.build_context(&query).await;

    assert!(context.is_empty());
}

/// 测试 format_question_with_context：context 为空时，应原样返回 question，不拼接前缀。
#[tokio::test]
async fn test_format_question_with_context_empty() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = new_test_handler(repo, memory_store, rx);

    let question = "What is AI?";
    let context = "";
    let result = handler.format_question_with_context(question, context);

    assert_eq!(result, question);
}

/// 测试 format_question_with_context：context 非空时，结果应包含 context、
/// 「用户提问:」以及 question，且顺序为 context + 用户提问 + question。
#[tokio::test]
async fn test_format_question_with_context_with_data() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = new_test_handler(repo, memory_store, rx);

    let question = "What is AI?";
    let context = "[回复消息]\n用户: test\n内容: Hello";
    let result = handler.format_question_with_context(question, context);

    assert!(result.contains(context));
    assert!(result.contains("用户提问:"));
    assert!(result.contains(question));
}

/// 测试 save_to_memory：以 User 角色写入一条内容后，MemoryStore 中应能按 user_id 搜到该条，
/// 且 content 与 role 正确。
#[tokio::test]
async fn test_save_to_memory_user_query() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let handler = new_test_handler(repo, memory_store.clone(), _rx);

    let query = AIQuery {
        chat_id: 123,
        user_id: 456,
        question: "What is AI?".to_string(),
        reply_to_message_id: None,
    };

    handler.save_to_memory(&query, "What is AI?", MemoryRole::User).await;

    let entries = memory_store.search_by_user("456").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "What is AI?");
    assert_eq!(entries[0].metadata.role, MemoryRole::User);
}

/// 测试 save_to_memory：以 Assistant 角色写入一条内容后，MemoryStore 中应能按 user_id 搜到，
/// 且 content 与 role 为 Assistant。
#[tokio::test]
async fn test_save_to_memory_ai_response() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let handler = new_test_handler(repo, memory_store.clone(), _rx);

    let query = AIQuery {
        chat_id: 123,
        user_id: 456,
        question: "What is AI?".to_string(),
        reply_to_message_id: None,
    };

    handler
        .save_to_memory(&query, "AI is artificial intelligence.", MemoryRole::Assistant)
        .await;

    let entries = memory_store.search_by_user("456").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "AI is artificial intelligence.");
    assert_eq!(entries[0].metadata.role, MemoryRole::Assistant);
}

/// 测试 build_memory_context：当记忆为空（无该 user/conversation 历史）时，应返回空字符串。
#[tokio::test]
async fn test_build_memory_context_empty() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let handler = new_test_handler(repo, memory_store, _rx);

    let context: String = handler.build_memory_context("123", "456", "").await;
    assert!(context.is_empty());
}

/// 测试 build_memory_context：先通过 save_to_memory 写入一条用户消息，再对该 user/conversation 构建上下文，
/// 应得到非空字符串且包含刚写入的内容。
#[tokio::test]
async fn test_build_memory_context_with_history() {
    let database_url = "sqlite::memory:";
    let repo = MessageRepository::new(database_url)
        .await
        .expect("Failed to create repository");

    let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

    let query = AIQuery {
        chat_id: 456,
        user_id: 123,
        question: "Hello".to_string(),
        reply_to_message_id: None,
    };

    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let handler = new_test_handler(repo, memory_store.clone(), _rx);

    handler.save_to_memory(&query, "What is AI?", MemoryRole::User).await;

    let context: String = handler.build_memory_context("123", "456", &query.question).await;
    assert!(!context.is_empty());
    assert!(context.contains("What is AI?"));
}
