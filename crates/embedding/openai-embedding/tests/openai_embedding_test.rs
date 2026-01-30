//! Integration tests for the OpenAI embedding service.
//!
//! These tests exercise the [`openai_embedding::OpenAIEmbedding`] implementation against
//! the real OpenAI embedding API. Tests that call the API are marked with `#[ignore]` and
//! require the `OPENAI_API_KEY` environment variable (and sufficient quota).
//!
//! # Running tests
//!
//! - **Default (no API):** `cargo test -p openai-embedding` — runs only tests that do not call the API.
//! - **With API:** `cargo test -p openai-embedding -- --ignored` — runs ignored tests; set
//!   `OPENAI_API_KEY` (e.g. in repo root `.env`). Quota/billing errors are treated as skip, not failure.

use std::path::Path;

use openai_embedding::OpenAIEmbedding;

/// Loads `.env` from the workspace root so `OPENAI_API_KEY` is available in ignored tests.
/// Path: `crates/embedding/openai-embedding` → `../../../.env` = repo root.
fn load_root_env() {
    let root_env = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.env");
    let _ = dotenvy::from_path(root_env);
}

/// Returns true if the error is due to OpenAI quota/billing/rate-limit; such tests are skipped instead of failed.
fn is_quota_or_billing_error(e: &anyhow::Error) -> bool {
    let s = e.to_string();
    s.contains("insufficient_quota")
        || s.contains("quota")
        || s.contains("billing")
        || s.contains("rate_limit")
}

/// **Test: Single-text embedding (real API).**
///
/// **Setup:** Loads env from workspace root, reads `OPENAI_API_KEY`, builds `OpenAIEmbedding`
/// with model `text-embedding-3-small`.
///
/// **Action:** Calls `embed("Hello world")`.
///
/// **Expected:** Returns a non-empty embedding vector of length 1536. If the error is
/// quota/billing/rate-limit, the test is skipped (eprint and return); otherwise the test fails.
#[tokio::test]
#[ignore] // Requires API key and quota, run with: cargo test -p openai-embedding -- --ignored
async fn test_openai_embedding() {
    load_root_env();
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable must be set for this test (or set in root .env)");

    let service = OpenAIEmbedding::new(api_key, "text-embedding-3-small".to_string());

    match service.embed("Hello world").await {
        Ok(embedding) => {
            assert!(!embedding.is_empty());
            assert_eq!(embedding.len(), 1536); // text-embedding-3-small produces 1536 dimensions
        }
        Err(e) if is_quota_or_billing_error(&e) => {
            eprintln!("test_openai_embedding skipped: OpenAI quota/billing limit ({})", e);
        }
        Err(e) => panic!("OpenAI embed request failed: {}", e),
    }
}

/// **Test: Batch embedding (real API).**
///
/// **Setup:** Same as single-text test; service with `text-embedding-3-small`.
///
/// **Action:** Calls `embed_batch` with three strings: "Hello", "World", "Goodbye".
///
/// **Expected:** Returns exactly three embedding vectors, each non-empty and of length 1536.
/// Quota/billing errors cause the test to be skipped.
#[tokio::test]
#[ignore]
async fn test_openai_embedding_batch() {
    load_root_env();
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable must be set for this test (or set in root .env)");

    let service = OpenAIEmbedding::new(api_key, "text-embedding-3-small".to_string());

    let texts = vec![
        "Hello".to_string(),
        "World".to_string(),
        "Goodbye".to_string(),
    ];

    match service.embed_batch(&texts).await {
        Ok(embeddings) => {
            assert_eq!(embeddings.len(), 3);
            for embedding in embeddings {
                assert!(!embedding.is_empty());
                assert_eq!(embedding.len(), 1536);
            }
        }
        Err(e) if is_quota_or_billing_error(&e) => {
            eprintln!("test_openai_embedding_batch skipped: OpenAI quota/billing limit ({})", e);
        }
        Err(e) => panic!("OpenAI embed_batch request failed: {}", e),
    }
}

/// **Test: Construction from empty API key (no API call).**
///
/// **Setup:** None; no env required.
///
/// **Action:** Creates `OpenAIEmbedding::with_api_key(String::new())`.
///
/// **Expected:** Does not panic; `model()` returns `"text-embedding-3-small"`. Actual API call would fail without key.
#[tokio::test]
async fn test_openai_embedding_from_env() {
    // Should not panic even without API key (will fail on actual API call)
    let service = OpenAIEmbedding::with_api_key(String::new());
    assert_eq!(service.model(), "text-embedding-3-small");
}
