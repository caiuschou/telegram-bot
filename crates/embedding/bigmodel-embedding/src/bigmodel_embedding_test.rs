//! Unit tests for BigModel embedding service.
//!
//! Tests single embed, batch embed, and construction from env.
//! Integration tests that call the real API are marked `#[ignore]` and require BIGMODEL_API_KEY;
//! run with: `cargo test -p bigmodel-embedding -- --ignored`
//!
//! Env: loads workspace root `.env` (from CARGO_MANIFEST_DIR/../../../.env) so BIGMODEL_API_KEY
//! can be read when running tests from the repo root or from the crate directory.

use std::path::Path;

use super::*;

/// Load `.env` from workspace root so BIGMODEL_API_KEY is available in ignored tests.
/// Path: crates/embedding/bigmodel-embedding -> ../../../.env = repo root.
fn load_root_env() {
    let root_env = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.env");
    let _ = dotenvy::from_path(root_env);
}

#[tokio::test]
#[ignore] // Requires API key, run with: cargo test -p bigmodel-embedding -- --ignored
async fn test_bigmodel_embedding() {
    load_root_env();
    let api_key = std::env::var("BIGMODEL_API_KEY")
        .expect("BIGMODEL_API_KEY environment variable must be set for this test (or set in root .env)");

    let service = BigModelEmbedding::new(api_key, "embedding-2".to_string());

    let embedding = service.embed("Hello world").await.unwrap();
    assert!(!embedding.is_empty());
    assert_eq!(embedding.len(), 1024); // embedding-2 produces 1024 dimensions
}

#[tokio::test]
#[ignore]
async fn test_bigmodel_embedding_chinese() {
    load_root_env();
    let api_key = std::env::var("BIGMODEL_API_KEY")
        .expect("BIGMODEL_API_KEY environment variable must be set for this test (or set in root .env)");

    let service = BigModelEmbedding::with_api_key(api_key);

    let embedding = service.embed("你好世界").await.unwrap();
    assert!(!embedding.is_empty());
    assert_eq!(embedding.len(), 1024);
}

#[tokio::test]
#[ignore]
async fn test_bigmodel_embedding_batch() {
    load_root_env();
    let api_key = std::env::var("BIGMODEL_API_KEY")
        .expect("BIGMODEL_API_KEY environment variable must be set for this test (or set in root .env)");

    let service = BigModelEmbedding::new(api_key, "embedding-2".to_string());

    let texts = vec![
        "Hello".to_string(),
        "World".to_string(),
        "Goodbye".to_string(),
    ];

    let embeddings = service.embed_batch(&texts).await.unwrap();
    assert_eq!(embeddings.len(), 3);
    for embedding in embeddings {
        assert!(!embedding.is_empty());
        assert_eq!(embedding.len(), 1024);
    }
}

#[tokio::test]
async fn test_bigmodel_embedding_from_env() {
    // Should not panic even without API key (will fail on actual API call)
    let service = BigModelEmbedding::with_api_key(String::new());
    assert_eq!(service.model(), "embedding-2");
}
