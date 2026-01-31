//! Vector index types for LanceVectorStore.

#[derive(Debug, Clone)]
pub enum LanceIndexType {
    Auto,
    IvfPq,
    Hnsw,
}
