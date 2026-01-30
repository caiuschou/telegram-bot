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
/// | `use_exact_search` | `bool` | If true, skip vector index (exhaustive flat search); higher accuracy, slower at scale |
/// | `refine_factor` | `Option<u32>` | For IVF-PQ index: fetch limit×refine_factor candidates then reorder by true distance; higher = more accurate |
/// | `nprobes` | `Option<usize>` | Number of IVF partitions to search; higher = better recall, slower (Lance default 20) |
/// | `semantic_fetch_multiplier` | `u32` | When filtering by user/conversation, fetch_limit = limit × this; default 10 |
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
    /// If true, skip vector index (exhaustive flat search). Use for maximum accuracy on small/medium tables.
    pub use_exact_search: bool,
    /// For IVF-PQ index: multiplier for refine step (limit × refine_factor candidates then reorder). None = use Lance default.
    pub refine_factor: Option<u32>,
    /// Number of IVF partitions to search. None = use Lance default (20).
    pub nprobes: Option<usize>,
    /// When filtering by user_id/conversation_id, fetch_limit = limit × this (min 50). Ensures enough candidates after filter.
    pub semantic_fetch_multiplier: u32,
}

impl Default for LanceConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/lancedb".to_string(),
            table_name: "memories".to_string(),
            embedding_dim: 1536, // OpenAI text-embedding-ada-002
            distance_type: DistanceType::Cosine,
            use_exact_search: false,
            refine_factor: None,
            nprobes: None,
            semantic_fetch_multiplier: 10,
        }
    }
}
