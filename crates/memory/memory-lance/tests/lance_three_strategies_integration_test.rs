//! Lance + ContextBuilder three-strategy integration test
//!
//! Uses real Lance store and Zhipu embedding to verify ContextBuilder runs
//! RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy in sequence:
//! - Write same-session entries (cat/dog/car) and preference text (I like / I prefer)
//! - Assert recent_messages / semantic_messages contain recent and semantic hits (including cat)
//! - Assert user_preferences is non-empty with preference keywords
//!
//! External: temp dir Lance DB; ContextBuilder, strategies, MemoryStore, EmbeddingService;
//! Zhipu API (BIGMODEL_API_KEY or ZHIPUAI_API_KEY, skip if unset).

use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

use bigmodel_embedding::BigModelEmbedding;
use embedding::EmbeddingService;
use memory_core::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use memory_strategies::{
    RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy,
};
use telegram_bot::memory::ContextBuilder;
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

/// Verify: Lance store + ContextBuilder runs three strategies (RecentMessages + SemanticSearch + UserPreferences)
///
/// Steps:
/// 1. Use Zhipu BigModelEmbedding for vectors; skip if no API key
/// 2. Create temp Lance DB and write: same-session cat/dog/car entries (with vectors) + preference "I like pizza and I prefer tea"
/// 3. ContextBuilder with RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy
/// 4. build(); assert recent_messages / semantic_messages contain recent and semantic hits (cat), user_preferences non-empty with keywords
#[tokio::test]
async fn test_lance_all_three_strategies_build_context() {
    let embedding = match make_zhipu_embedding() {
        Some(svc) => svc,
        None => return,
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("lance_three_strategies_db");
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
    let content_pref = "I like pizza and I prefer tea";

    let emb_cat = embedding.embed(content_cat).await.expect("embed cat");
    let emb_dog = embedding.embed(content_dog).await.expect("embed dog");
    let emb_car = embedding.embed(content_car).await.expect("embed car");
    let emb_pref = embedding.embed(content_pref).await.expect("embed pref");

    let meta_conv = |role: MemoryRole| MemoryMetadata {
        user_id: Some("u1".to_string()),
        conversation_id: Some("c1".to_string()),
        role,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(1.0),
    };

    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_cat.to_string(),
            embedding: Some(emb_cat),
            metadata: meta_conv(MemoryRole::User),
        })
        .await
        .expect("add cat");
    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_dog.to_string(),
            embedding: Some(emb_dog),
            metadata: meta_conv(MemoryRole::User),
        })
        .await
        .expect("add dog");
    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_car.to_string(),
            embedding: Some(emb_car),
            metadata: meta_conv(MemoryRole::Assistant),
        })
        .await
        .expect("add car");
    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_pref.to_string(),
            embedding: Some(emb_pref),
            metadata: meta_conv(MemoryRole::User),
        })
        .await
        .expect("add pref");

    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);

    let context = ContextBuilder::new(store_arc)
        .with_strategy(Box::new(RecentMessagesStrategy::new(10)))
        .with_strategy(Box::new(SemanticSearchStrategy::new(3, embedding, 0.0)))
        .with_strategy(Box::new(UserPreferencesStrategy::new()))
        .for_user("u1")
        .for_conversation("c1")
        .with_query("User asks: what do cats eat?")
        .build()
        .await
        .expect("build context");

    assert!(
        !context.is_empty(),
        "context should contain recent and/or semantic messages"
    );
    // RecentMessagesStrategy should return 4 recent messages for conversation c1
    assert!(
        context.recent_messages.len() >= 4,
        "recent_messages should contain at least 4 from RecentMessagesStrategy, got {}",
        context.recent_messages.len()
    );
    let recent_joined = context.recent_messages.join(" ");
    assert!(
        recent_joined.contains("dogs") || recent_joined.contains("loyal"),
        "recent_messages should include message about dogs, got: {}",
        recent_joined
    );
    assert!(
        recent_joined.to_lowercase().contains("pizza") || recent_joined.to_lowercase().contains("tea"),
        "recent_messages should include preference (I like pizza / I prefer tea), got: {}",
        recent_joined
    );
    // SemanticSearchStrategy should return semantic hit about cat for query
    let semantic_joined = context.semantic_messages.join(" ");
    assert!(
        semantic_joined.contains("cat") || semantic_joined.contains("cats"),
        "semantic_messages should include hit about cat, got: {}",
        semantic_joined
    );
    assert!(
        context.user_preferences.is_some(),
        "user_preferences should be set by UserPreferencesStrategy"
    );
    let prefs = context.user_preferences.as_deref().unwrap();
    assert!(
        prefs.to_lowercase().contains("like") || prefs.to_lowercase().contains("prefer"),
        "user_preferences should contain preference keywords, got: {}",
        prefs
    );
}
