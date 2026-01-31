//! Configuration for LanceVectorStore.

use super::DistanceType;

#[derive(Debug, Clone)]
pub struct LanceConfig {
    pub db_path: String,
    pub table_name: String,
    pub embedding_dim: usize,
    pub distance_type: DistanceType,
    pub use_exact_search: bool,
    pub refine_factor: Option<u32>,
    pub nprobes: Option<usize>,
    pub semantic_fetch_multiplier: u32,
}

impl Default for LanceConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/lancedb".to_string(),
            table_name: "memories".to_string(),
            embedding_dim: 1536,
            distance_type: DistanceType::Cosine,
            use_exact_search: false,
            refine_factor: None,
            nprobes: None,
            semantic_fetch_multiplier: 10,
        }
    }
}
