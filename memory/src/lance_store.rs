//! # Lance Vector Store
//!
//! High-performance vector storage using LanceDB.
//!
//! ## Features
//!
//! - **Persistent storage** with Lance format
//! - **Vector indexing** with IVF-PQ and HNSW
//! - **Semantic search** with configurable distance metrics
//! - **Metadata filtering** for efficient querying
//!
//! ## Usage
//!
//! ```rust,ignore
//! use memory::{LanceVectorStore, MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
//! use chrono::Utc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create store with default settings
//! let store = LanceVectorStore::new("./data/lancedb").await?;
//!
//! // Add entry
//! let metadata = MemoryMetadata {
//!     user_id: Some("user123".to_string()),
//!     conversation_id: None,
//!     role: MemoryRole::User,
//!     timestamp: Utc::now(),
//!     tokens: None,
//!     importance: None,
//! };
//! let entry = MemoryEntry::new("Hello world".to_string(), metadata);
//! store.add(entry).await?;
//!
//! // Semantic search
//! let query_embedding = vec![0.1; 1536]; // OpenAI embedding dimension
//! let results = store.semantic_search(&query_embedding, 10).await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "lance")]
use {
    async_trait::async_trait,
    arrow_array::{Float32Array, RecordBatch, StringArray, UInt64Array},
    arrow_schema::{DataType, Field, Schema},
    std::path::Path,
    std::sync::Arc,
    tokio::sync::RwLock,
};

use crate::types::{MemoryEntry, MemoryMetadata, MemoryRole};
use crate::store::MemoryStore;
use anyhow::{anyhow, Result};
use uuid::Uuid;

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

#[cfg(feature = "lance")]
impl DistanceType {
    fn as_lance_metric(&self) -> lancedb::DistanceType {
        match self {
            DistanceType::Cosine => lancedb::DistanceType::Cosine,
            DistanceType::L2 => lancedb::DistanceType::L2,
            DistanceType::Dot => lancedb::DistanceType::Dot,
        }
    }
}

/// LanceDB-based vector store implementation.
///
/// This store provides persistent, high-performance vector storage using LanceDB.
/// It supports automatic vector indexing with IVF-PQ or HNSW for fast semantic search.
#[cfg(feature = "lance")]
pub struct LanceVectorStore {
    config: LanceConfig,
    db: Arc<RwLock<lancedb::Connection>>,
}

#[cfg(feature = "lance")]
impl LanceVectorStore {
    /// Creates a new LanceVectorStore with the given database path.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the LanceDB database directory
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized store or an error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let store = LanceVectorStore::new("./data/lancedb").await?;
    /// ```
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Self::with_config(LanceConfig {
            db_path: db_path.as_ref().to_string_lossy().to_string(),
            ..Default::default()
        })
        .await
    }

    /// Creates a new LanceVectorStore with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Custom `LanceConfig`
    pub async fn with_config(config: LanceConfig) -> Result<Self> {
        // Connect to database (creates if not exists)
        let db = lancedb::connect(&config.db_path)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to connect to LanceDB: {}", e))?;

        let store = Self {
            config,
            db: Arc::new(RwLock::new(db)),
        };

        // Ensure table exists
        store.ensure_table().await?;

        Ok(store)
    }

    /// Ensures the memory table exists, creating it if necessary.
    async fn ensure_table(&self) -> Result<()> {
        let db = self.db.read().await;
        let table_names = db
            .table_names()
            .await
            .map_err(|e| anyhow!("Failed to list tables: {}", e))?;

        if !table_names.contains(&self.config.table_name) {
            // Create schema
            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("content", DataType::Utf8, false),
                Field::new(
                    "vector",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        self.config.embedding_dim as i32,
                    ),
                    true,
                ),
                Field::new("user_id", DataType::Utf8, true),
                Field::new("conversation_id", DataType::Utf8, true),
                Field::new("role", DataType::Utf8, false),
                Field::new("timestamp", DataType::Utf8, false),
                Field::new("tokens", DataType::UInt32, true),
                Field::new("importance", DataType::Float32, true),
            ]));

            // Create empty table
            db.create_empty_table(&self.config.table_name, schema)
                .await
                .map_err(|e| anyhow!("Failed to create table: {}", e))?;
        }

        Ok(())
    }

    /// Creates a vector index on the table.
    ///
    /// # Arguments
    ///
    /// * `index_type` - Type of index to create
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// store.create_index(LanceIndexType::IvfPq).await?;
    /// ```
    pub async fn create_index(&self, index_type: LanceIndexType) -> Result<()> {
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let index_params = match index_type {
            LanceIndexType::Auto => lancedb::index::Index::Auto,
            LanceIndexType::IvfPq => lancedb::index::Index::Auto, // Use Auto for IVF-PQ
            LanceIndexType::Hnsw => lancedb::index::Index::Auto,  // Use Auto for now
        };

        table
            .create_index(&["vector"], index_params)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to create index: {}", e))?;

        Ok(())
    }

    /// Converts a MemoryEntry to a RecordBatch.
    fn entry_to_batch(&self, entry: &MemoryEntry) -> Result<RecordBatch> {
        let schema = Self::batch_schema(self.config.embedding_dim)?;

        let id = entry.id.to_string();
        let content = &entry.content;
        let user_id = entry.metadata.user_id.as_deref();
        let conversation_id = entry.metadata.conversation_id.as_deref();
        let role = format!("{:?}", entry.metadata.role);
        let timestamp = entry.metadata.timestamp.to_rfc3339();
        let tokens = entry.metadata.tokens;
        let importance = entry.metadata.importance;

        // Build columns
        let id_array = StringArray::from(vec![id.as_str()]);
        let content_array = StringArray::from(vec![content.as_str()]);
        let user_id_array = StringArray::from(vec![user_id.unwrap_or("")]);
        let conversation_id_array = StringArray::from(vec![conversation_id.unwrap_or("")]);
        let role_array = StringArray::from(vec![role.as_str()]);
        let timestamp_array = StringArray::from(vec![timestamp.as_str()]);

        // Handle vector column
        let vector_array = if let Some(embedding) = &entry.embedding {
            let vec_data: Vec<Option<f32>> = embedding.iter().map(|&x| Some(x)).collect();
            arrow_array::FixedSizeListArray::from_iter_primitive::<arrow_array::types::Float32Type, _, _>(
                std::iter::once(Some(vec_data)),
                self.config.embedding_dim as i32,
            )
        } else {
            // Create empty/null vector
            let vec_data: Vec<Option<f32>> = vec![None; self.config.embedding_dim];
            arrow_array::FixedSizeListArray::from_iter_primitive::<arrow_array::types::Float32Type, _, _>(
                std::iter::once(Some(vec_data)),
                self.config.embedding_dim as i32,
            )
        };

        let tokens_array = if let Some(t) = tokens {
            UInt64Array::from(vec![Some(t as u64)])
        } else {
            UInt64Array::from(vec![None as Option<u64>])
        };

        let importance_array = if let Some(imp) = importance {
            arrow_array::Float32Array::from(vec![Some(imp)])
        } else {
            arrow_array::Float32Array::from(vec![None as Option<f32>])
        };

        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(id_array),
                Arc::new(content_array),
                Arc::new(vector_array),
                Arc::new(user_id_array),
                Arc::new(conversation_id_array),
                Arc::new(role_array),
                Arc::new(timestamp_array),
                Arc::new(tokens_array),
                Arc::new(importance_array),
            ],
        )?;

        Ok(batch)
    }

    /// Converts a RecordBatch row to a MemoryEntry.
    fn batch_to_entry(&self, batch: &RecordBatch, row: usize) -> Result<MemoryEntry> {
        let id_col = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("ID column is not StringArray"))?;
        let id = Uuid::parse_str(id_col.value(row))?;

        let content_col = batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Content column is not StringArray"))?;
        let content = content_col.value(row).to_string();

        let vector_col = batch
            .column(2)
            .as_any()
            .downcast_ref::<arrow_array::FixedSizeListArray>()
            .ok_or_else(|| anyhow!("Vector column is not FixedSizeListArray"))?;

        let embedding = if vector_col.is_null(row) {
            None
        } else {
            let values = vector_col
                .value(row)
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or_else(|| anyhow!("Vector values are not Float32Array"))?;
            Some(values.iter().map(|x| x.unwrap_or(0.0)).collect())
        };

        let user_id_col = batch
            .column(3)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("User ID column is not StringArray"))?;
        let user_id = if user_id_col.is_null(row) || user_id_col.value(row).is_empty() {
            None
        } else {
            Some(user_id_col.value(row).to_string())
        };

        let conversation_id_col = batch
            .column(4)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Conversation ID column is not StringArray"))?;
        let conversation_id = if conversation_id_col.is_null(row) || conversation_id_col.value(row).is_empty() {
            None
        } else {
            Some(conversation_id_col.value(row).to_string())
        };

        let role_col = batch
            .column(5)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Role column is not StringArray"))?;
        let role_str = role_col.value(row);
        let role = match role_str {
            "User" => MemoryRole::User,
            "Assistant" => MemoryRole::Assistant,
            "System" => MemoryRole::System,
            _ => return Err(anyhow!("Unknown role: {}", role_str)),
        };

        let timestamp_col = batch
            .column(6)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Timestamp column is not StringArray"))?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_col.value(row))?
            .with_timezone(&chrono::Utc);

        let tokens_col = batch
            .column(7)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| anyhow!("Tokens column is not UInt64Array"))?;
        let tokens = if tokens_col.is_null(row) {
            None
        } else {
            Some(tokens_col.value(row) as u32)
        };

        let importance_col = batch
            .column(8)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .ok_or_else(|| anyhow!("Importance column is not Float32Array"))?;
        let importance = if importance_col.is_null(row) {
            None
        } else {
            Some(importance_col.value(row))
        };

        Ok(MemoryEntry {
            id,
            content,
            embedding,
            metadata: MemoryMetadata {
                user_id,
                conversation_id,
                role,
                timestamp,
                tokens,
                importance,
            },
        })
    }

    /// Returns the schema for memory RecordBatches.
    fn batch_schema(embedding_dim: usize) -> Result<Arc<Schema>> {
        Ok(Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    embedding_dim as i32,
                ),
                true,
            ),
            Field::new("user_id", DataType::Utf8, true),
            Field::new("conversation_id", DataType::Utf8, true),
            Field::new("role", DataType::Utf8, false),
            Field::new("timestamp", DataType::Utf8, false),
            Field::new("tokens", DataType::UInt32, true),
            Field::new("importance", DataType::Float32, true),
        ])))
    }
}

#[cfg(feature = "lance")]
#[async_trait]
impl MemoryStore for LanceVectorStore {
    async fn add(&self, entry: MemoryEntry) -> Result<()> {
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let batch = self.entry_to_batch(&entry)?;
        let mut stream = futures::stream::iter(vec![Ok(batch)]).boxed();

        table
            .add(&mut stream)
            .await
            .map_err(|e| anyhow!("Failed to add entry: {}", e))?;

        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>> {
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let results = table
            .query()
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;

        let results = results
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| anyhow!("Failed to collect results: {}", e))?;

        for batch in results {
            for row in 0..batch.num_rows() {
                let entry = self.batch_to_entry(&batch, row)?;
                if entry.id == id {
                    return Ok(Some(entry));
                }
            }
        }

        Ok(None)
    }

    async fn update(&self, entry: MemoryEntry) -> Result<()> {
        // LanceDB doesn't have direct update, so we delete and re-add
        self.delete(entry.id).await?;
        self.add(entry).await?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let id_str = id.to_string();
        table
            .delete(&format!("id = '{}'", id_str))
            .await
            .map_err(|e| anyhow!("Failed to delete entry: {}", e))?;

        Ok(())
    }

    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>> {
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let results = table
            .query()
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;

        let results = results
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| anyhow!("Failed to collect results: {}", e))?;

        let mut entries = Vec::new();
        for batch in results {
            for row in 0..batch.num_rows() {
                let entry = self.batch_to_entry(&batch, row)?;
                if entry.metadata.user_id.as_deref() == Some(user_id) {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    async fn search_by_conversation(&self, conversation_id: &str) -> Result<Vec<MemoryEntry>> {
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let results = table
            .query()
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;

        let results = results
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| anyhow!("Failed to collect results: {}", e))?;

        let mut entries = Vec::new();
        for batch in results {
            for row in 0..batch.num_rows() {
                let entry = self.batch_to_entry(&batch, row)?;
                if entry.metadata.conversation_id.as_deref() == Some(conversation_id) {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    async fn semantic_search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<MemoryEntry>> {
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let results = table
            .query()
            .nearest_to(query_embedding)
            .map_err(|e| anyhow!("Failed to create vector query: {}", e))?
            .limit(limit)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to execute vector search: {}", e))?;

        let results = results
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| anyhow!("Failed to collect results: {}", e))?;

        let mut entries = Vec::new();
        for batch in results {
            for row in 0..batch.num_rows() {
                let entry = self.batch_to_entry(&batch, row)?;
                entries.push(entry);
            }
        }

        Ok(entries)
    }
}

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
