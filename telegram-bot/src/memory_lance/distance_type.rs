//! Distance metrics for vector similarity search.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceType {
    Cosine,
    L2,
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
