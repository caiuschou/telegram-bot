# Text Embeddings

This document describes the embedding service interface and implementations.

## EmbeddingService Trait

The `EmbeddingService` trait defines the interface for generating text embeddings.

### Required Methods

#### `embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error>`

Generates an embedding vector for a single text string.

#### `embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error>`

Generates embedding vectors for multiple texts in a single API call. This is more efficient than calling `embed` multiple times.

### Implementations

#### OpenAIEmbedding

Uses OpenAI's embedding API (e.g., `text-embedding-3-small`, `text-embedding-3-large`).

**Advantages**:
- High quality embeddings
- Well-documented
- Multiple model options

**Considerations**:
- Requires API key
- Rate limits
- Cost per request

#### Future Implementations

- Local models (Sentence Transformers via ONNX Runtime)
- Other providers (Cohere, HuggingFace, etc.)

## Example Usage

```rust
use memory::EmbeddingService;

async fn example(service: &impl EmbeddingService) -> Result<(), anyhow::Error> {
    // Single text embedding
    let embedding = service.embed("Hello world").await?;
    println!("Embedding dimension: {}", embedding.len());
    
    // Batch embedding
    let texts = vec![
        "Hello".to_string(),
        "World".to_string(),
        "Goodbye".to_string(),
    ];
    let embeddings = service.embed_batch(&texts).await?;
    
    Ok(())
}
```

## Embedding Best Practices

### Batch Processing

Always use `embed_batch` when processing multiple texts to reduce API calls and costs.

### Dimensionality

Choose an embedding dimension based on your use case:
- **Small (384-768)**: Faster, lower cost, good for most use cases
- **Medium (1024-1536)**: Better semantic understanding
- **Large (3072+)**: Maximum accuracy, higher cost

### Normalization

Embeddings should be normalized for cosine similarity calculations.

### Caching

Consider caching embeddings for frequently used texts to reduce API calls.

## Integration with MemoryStore

The embedding service is typically used together with the memory store:

```rust
async fn add_with_embedding(
    store: &impl MemoryStore,
    embedding_service: &impl EmbeddingService,
    content: String,
    metadata: MemoryMetadata,
) -> Result<(), anyhow::Error> {
    let mut entry = MemoryEntry::new(content, metadata);
    
    // Generate embedding
    let embedding = embedding_service.embed(&entry.content).await?;
    entry.embedding = Some(embedding);
    
    // Store entry
    store.add(entry).await?;
    
    Ok(())
}
```

## Semantic Search

Embeddings enable semantic search by comparing vector similarity:

```rust
use std::collections::HashMap;

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    dot_product / (norm_a * norm_b)
}
```
