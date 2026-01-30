//! Distance metrics for vector similarity search.
//!
//! Maps to lancedb::DistanceType. Used in LanceConfig. External: lancedb.

/// Distance metrics for vector similarity search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceType {
    /// Cosine similarity (recommended for normalized embeddings)
    Cosine,
    /// Euclidean (L2) distance
    L2,
    /// Dot product
    Dot,
}

impl DistanceType {
    #[allow(dead_code)]
    pub(crate) fn as_lance_metric(&self) -> lancedb::DistanceType {
        match self {
            DistanceType::Cosine => lancedb::DistanceType::Cosine,
            DistanceType::L2 => lancedb::DistanceType::L2,
            DistanceType::Dot => lancedb::DistanceType::Dot,
        }
    }
}
