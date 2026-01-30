//! Lance + SemanticSearchStrategy integration test
//!
//! Uses real Lance store and Zhipu embedding to verify full strategy flow:
//! - BigModelEmbedding (Zhipu embedding-2) generates vectors for test data and writes to Lance
//! - SemanticSearchStrategy uses same Zhipu service to embed query and run semantic search
//! - Verify query about cat returns the message semantically nearest to "about cat"
//!
//! External: temp dir Lance DB; SemanticSearchStrategy, MemoryStore, EmbeddingService;
//! Zhipu API (BIGMODEL_API_KEY or ZHIPUAI_API_KEY, skip if unset).

use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

use bigmodel_embedding::BigModelEmbedding;
use embedding::EmbeddingService;
use memory::{
    ContextStrategy, MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, SemanticSearchStrategy,
    StrategyResult,
};
use memory_lance::{LanceConfig, LanceVectorStore};

/// Zhipu embedding-2 model dimension
const DIM: usize = 1024;

/// Get Zhipu API key from env; None if unset (test will skip).
fn zhipu_api_key() -> Option<String> {
    std::env::var("BIGMODEL_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("ZHIPUAI_API_KEY").ok().filter(|s| !s.is_empty()))
}

/// Create Zhipu embedding service (embedding-2); None if no API key.
fn make_zhipu_embedding() -> Option<Arc<BigModelEmbedding>> {
    zhipu_api_key().map(|key| Arc::new(BigModelEmbedding::new(key, "embedding-2".to_string())))
}

fn meta(role: MemoryRole) -> MemoryMetadata {
    MemoryMetadata {
        user_id: Some("u1".to_string()),
        conversation_id: Some("c1".to_string()),
        role,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(1.0),
    }
}

/// Verify: Lance store + SemanticSearchStrategy + Zhipu embedding
///
/// Steps:
/// 1. Use Zhipu BigModelEmbedding (embedding-2) for vectors; skip if no API key
/// 2. Create temp Lance DB and write three MemoryEntry with Zhipu vectors (cat, dog, car)
/// 3. build_context for query "User asks: what do cats eat?"; assert returned message contains "about cat" and is nearest
#[tokio::test]
async fn test_lance_semantic_strategy_returns_relevant_message() {
    let embedding = match make_zhipu_embedding() {
        Some(svc) => svc,
        None => return,
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("lance_semantic_db");
    let config = LanceConfig {
        db_path: db_path.to_string_lossy().to_string(),
        table_name: "memories".to_string(),
        embedding_dim: DIM,
        ..Default::default()
    };

    let store = LanceVectorStore::with_config(config)
        .await
        .expect("create LanceVectorStore");

    let content_cat = "Discussion about cats: cats are lovely pets.";
    let content_dog = "Discussion about dogs: dogs are loyal.";
    let content_car = "Discussion about cars: electric cars are eco-friendly.";

    let emb_cat = embedding.embed(content_cat).await.expect("embed cat");
    let emb_dog = embedding.embed(content_dog).await.expect("embed dog");
    let emb_car = embedding.embed(content_car).await.expect("embed car");

    let entry_cat = MemoryEntry {
        id: Uuid::new_v4(),
        content: content_cat.to_string(),
        embedding: Some(emb_cat),
        metadata: meta(MemoryRole::User),
    };
    let entry_dog = MemoryEntry {
        id: Uuid::new_v4(),
        content: content_dog.to_string(),
        embedding: Some(emb_dog),
        metadata: meta(MemoryRole::User),
    };
    let entry_car = MemoryEntry {
        id: Uuid::new_v4(),
        content: content_car.to_string(),
        embedding: Some(emb_car),
        metadata: meta(MemoryRole::Assistant),
    };

    store.add(entry_cat).await.expect("add cat");
    store.add(entry_dog).await.expect("add dog");
    store.add(entry_car).await.expect("add car");

    let strategy = SemanticSearchStrategy::new(3, embedding, 0.0);
    let result = strategy
        .build_context(&store, &None, &None, &Some("User asks: what do cats eat?".to_string()))
        .await
        .expect("build_context");

    let messages = match &result {
        StrategyResult::Messages { messages: m, .. } => m,
        StrategyResult::Empty => panic!("expected Messages, got Empty"),
        StrategyResult::Preferences(_) => panic!("expected Messages, got Preferences"),
    };

    assert!(
        !messages.is_empty(),
        "semantic search with Lance should return at least one message"
    );

    let first = &messages[0];
    assert!(
        first.contains("cat") || first.contains("cats"),
        "nearest to query about cat should be the cat entry, got: {}",
        first
    );
    assert!(
        first.contains("cats") || first.contains("cat"),
        "first message should be the cat discussion, got: {}",
        first
    );
}

/// Verify: strategy returns Empty when no query
#[tokio::test]
async fn test_lance_semantic_strategy_empty_query_returns_empty() {
    let embedding = match make_zhipu_embedding() {
        Some(svc) => svc,
        None => return,
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("lance_empty_query_db");
    let config = LanceConfig {
        db_path: db_path.to_string_lossy().to_string(),
        table_name: "memories".to_string(),
        embedding_dim: DIM,
        ..Default::default()
    };

    let store = LanceVectorStore::with_config(config)
        .await
        .expect("create LanceVectorStore");

    let content = "A message";
    let emb = embedding.embed(content).await.expect("embed");
    let entry = MemoryEntry {
        id: Uuid::new_v4(),
        content: content.to_string(),
        embedding: Some(emb),
        metadata: meta(MemoryRole::User),
    };
    store.add(entry).await.expect("add");

    let strategy = SemanticSearchStrategy::new(5, embedding, 0.0);
    let result = strategy
        .build_context(&store, &None, &None, &None)
        .await
        .expect("build_context");

    assert!(
        matches!(result, StrategyResult::Empty),
        "no query should yield Empty"
    );
}
