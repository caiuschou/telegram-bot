//! Integration tests for the BigModel embedding service.
//!
//! These tests exercise the [`bigmodel_embedding::BigModelEmbedding`] implementation against
//! the real BigModel (Zhipu AI) embedding API. Tests that call the API are marked with
//! `#[ignore]` and require the `BIGMODEL_API_KEY` environment variable.
//!
//! # Running tests
//!
//! - **Default (no API):** `cargo test -p bigmodel-embedding` — runs only tests that do not call the API.
//! - **With API:** `cargo test -p bigmodel-embedding -- --ignored` — runs ignored tests; set
//!   `BIGMODEL_API_KEY` (e.g. in repo root `.env`). The test helper loads `.env` from the
//!   workspace root so the key can be read when running from the repo root or from the crate directory.

use std::path::Path;

use embedding::EmbeddingService;
use bigmodel_embedding::BigModelEmbedding;

/// Loads `.env` from the workspace root so `BIGMODEL_API_KEY` is available in ignored tests.
/// Path: `crates/embedding/bigmodel-embedding` → `../../../.env` = repo root.
fn load_root_env() {
    let root_env = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.env");
    let _ = dotenvy::from_path(root_env);
}

/// **Test: Single-text embedding (real API).**
///
/// **Setup:** Loads env from workspace root, reads `BIGMODEL_API_KEY`, builds `BigModelEmbedding`
/// with model `embedding-2`.
///
/// **Action:** Calls `embed("Hello world")`.
///
/// **Expected:** Returns a non-empty embedding vector of length 1024 (embedding-2 dimension).
///
/// **Note:** Ignored by default; run with `cargo test -p bigmodel-embedding -- --ignored`.
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

/// **Test: Single-text embedding for Chinese (real API).**
///
/// **Setup:** Same as `test_bigmodel_embedding` but uses `with_api_key` (default model).
///
/// **Action:** Calls `embed("你好世界")`.
///
/// **Expected:** Returns a non-empty embedding of length 1024; validates Chinese-optimized behavior.
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

/// **Test: Batch embedding (real API).**
///
/// **Setup:** Same as single-text test; service with `embedding-2`.
///
/// **Action:** Calls `embed_batch` with three strings: "Hello", "World", "Goodbye".
///
/// **Expected:** Returns exactly three embedding vectors, each non-empty and of length 1024.
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

/// **Test: Construction from empty API key (no API call).**
///
/// **Setup:** None; no env required.
///
/// **Action:** Creates `BigModelEmbedding::with_api_key(String::new())` (will use env at call time).
///
/// **Expected:** Does not panic; `model()` returns `"embedding-2"`. Actual API call would fail without key.
#[tokio::test]
async fn test_bigmodel_embedding_from_env() {
    // Should not panic even without API key (will fail on actual API call)
    let service = BigModelEmbedding::with_api_key(String::new());
    assert_eq!(service.model(), "embedding-2");
}
