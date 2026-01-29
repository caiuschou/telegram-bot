//! Configuration for LanceVectorStore.
//!
//! Defines LanceConfig and its default values. Used when creating a store
//! via `LanceVectorStore::with_config`. External: memory-lance public API.

use crate::DistanceType;

/// Configuration for LanceVectorStore.
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `db_path` | `String` | Path to the LanceDB database |
/// | `table_name` | `String` | Name of the table to use |
/// | `embedding_dim` | `usize` | Dimension of the embedding vectors |
/// | `distance_type` | `DistanceType` | Distance metric for vector search |
#[derive(Debug, Clone)]
pub struct LanceConfig {
    /// Path to the LanceDB database directory
    pub db_path: String,
    /// Name of the table to use/create
    pub table_name: String,
    /// Dimension of embedding vectors
    pub embedding_dim: usize,
    /// Distance metric for vector search
    pub distance_type: DistanceType,
}

impl Default for LanceConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/lancedb".to_string(),
            table_name: "memories".to_string(),
            embedding_dim: 1536, // OpenAI text-embedding-ada-002
            distance_type: DistanceType::Cosine,
        }
    }
}
