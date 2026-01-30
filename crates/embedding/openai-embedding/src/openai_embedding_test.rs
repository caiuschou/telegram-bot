//! Unit tests for OpenAI embedding service.
//!
//! Tests single embed, batch embed, and construction from env.
//! Integration tests that call the real API are marked `#[ignore]` and require OPENAI_API_KEY;
//! run with: `cargo test -p openai-embedding -- --ignored`
//!
//! Env: loads workspace root `.env` (from CARGO_MANIFEST_DIR/../../../.env) so OPENAI_API_KEY
//! can be read when running tests from the repo root or from the crate directory.

use std::path::Path;

use super::*;

/// Load `.env` from workspace root so OPENAI_API_KEY is available in ignored tests.
/// Path: crates/embedding/openai-embedding -> ../../../.env = repo root.
fn load_root_env() {
    let root_env = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.env");
    let _ = dotenvy::from_path(root_env);
}

/// True if the error is due to OpenAI quota/billing (test is skipped instead of failed).
fn is_quota_or_billing_error(e: &anyhow::Error) -> bool {
    let s = e.to_string();
    s.contains("insufficient_quota")
        || s.contains("quota")
        || s.contains("billing")
        || s.contains("rate_limit")
}

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

#[tokio::test]
async fn test_openai_embedding_from_env() {
    // Should not panic even without API key (will fail on actual API call)
    let service = OpenAIEmbedding::with_api_key(String::new());
    assert_eq!(service.model(), "text-embedding-3-small");
}
