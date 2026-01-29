//! Vector index types supported by LanceVectorStore.
//!
//! Used when calling `LanceVectorStore::create_index`. External: lancedb index API.

/// Vector index types supported by LanceVectorStore.
#[derive(Debug, Clone)]
pub enum LanceIndexType {
    /// Automatically choose the best index type
    Auto,
    /// IVF-PQ (Inverted File with Product Quantization)
    /// Good balance of speed and accuracy for large datasets
    IvfPq,
    /// HNSW (Hierarchical Navigable Small World)
    /// Fastest query performance, higher memory usage
    Hnsw,
}
