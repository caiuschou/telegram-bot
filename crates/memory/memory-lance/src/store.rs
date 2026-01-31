//! LanceDB-based vector store implementation.
//!
//! Provides persistent vector storage, RecordBatch conversion, and MemoryStore trait impl.
//! External: memory (MemoryStore, MemoryEntry), lancedb, arrow.

use async_trait::async_trait;
use arrow_array::{Array, Float32Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_array::types::Float32Type;
use arrow_schema::{DataType, Field, Schema};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use lancedb::query::{ExecutableQuery, QueryBase};
use futures::TryStreamExt;
use anyhow::{anyhow, Result};
use tracing::{error, info};
use uuid::Uuid;

use crate::config::LanceConfig;
use crate::index_type::LanceIndexType;

/// LanceDB-based vector store implementation.
///
/// This store provides persistent, high-performance vector storage using LanceDB.
/// It supports automatic vector indexing with IVF-PQ or HNSW for fast semantic search.
pub struct LanceVectorStore {
    pub(crate) config: LanceConfig,
    db: Arc<RwLock<lancedb::Connection>>,
}

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
            .execute()
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
                .execute()
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
            .execute()
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
            arrow_array::FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                std::iter::once(Some(vec_data)),
                self.config.embedding_dim as i32,
            )
        } else {
            // Create empty/null vector
            let vec_data: Vec<Option<f32>> = vec![None; self.config.embedding_dim];
            arrow_array::FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                std::iter::once(Some(vec_data)),
                self.config.embedding_dim as i32,
            )
        };

        let tokens_array = if let Some(t) = tokens {
            arrow_array::UInt32Array::from(vec![Some(t)])
        } else {
            arrow_array::UInt32Array::from(vec![None as Option<u32>])
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
    /// Uses column names from the batch schema so that reading is correct even if LanceDB returns columns in a different order (e.g. alphabetical).
    fn batch_to_entry(&self, batch: &RecordBatch, row: usize) -> Result<MemoryEntry> {
        let schema = batch.schema();
        let col = |name: &str| {
            schema
                .index_of(name)
                .map_err(|e| anyhow!("schema missing column {:?}: {}", name, e))
                .and_then(|i| Ok(batch.column(i).clone()))
        };

        let id_arc = col("id")?;
        let id_col = id_arc.as_ref().as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("ID column is not StringArray"))?;
        let id = Uuid::parse_str(id_col.value(row))?;

        let content_arc = col("content")?;
        let content_col = content_arc.as_ref().as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Content column is not StringArray"))?;
        let content = content_col.value(row).to_string();

        let vector_arc = col("vector")?;
        let vector_col = vector_arc.as_ref().as_any().downcast_ref::<arrow_array::FixedSizeListArray>()
            .ok_or_else(|| anyhow!("Vector column is not FixedSizeListArray"))?;

        let embedding = if vector_col.is_null(row) {
            None
        } else {
            let array = vector_col.value(row);
            let values = array
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or_else(|| anyhow!("Vector values are not Float32Array"))?;
            Some(values.iter().map(|x| x.unwrap_or(0.0)).collect())
        };

        let user_id_arc = col("user_id")?;
        let user_id_col = user_id_arc.as_ref().as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("User ID column is not StringArray"))?;
        let user_id = if user_id_col.is_null(row) || user_id_col.value(row).is_empty() {
            None
        } else {
            Some(user_id_col.value(row).to_string())
        };

        let conversation_id_arc = col("conversation_id")?;
        let conversation_id_col = conversation_id_arc.as_ref().as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Conversation ID column is not StringArray"))?;
        let conversation_id = if conversation_id_col.is_null(row) || conversation_id_col.value(row).is_empty() {
            None
        } else {
            Some(conversation_id_col.value(row).to_string())
        };

        let role_arc = col("role")?;
        let role_col = role_arc.as_ref().as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Role column is not StringArray"))?;
        let role_str = role_col.value(row);
        let role = match role_str {
            "User" => MemoryRole::User,
            "Assistant" => MemoryRole::Assistant,
            "System" => MemoryRole::System,
            _ => return Err(anyhow!("Unknown role: {}", role_str)),
        };

        let timestamp_arc = col("timestamp")?;
        let timestamp_col = timestamp_arc.as_ref().as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Timestamp column is not StringArray"))?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_col.value(row))?
            .with_timezone(&chrono::Utc);

        let tokens_arc = col("tokens")?;
        let tokens_col = tokens_arc.as_ref().as_any().downcast_ref::<arrow_array::UInt32Array>()
            .ok_or_else(|| anyhow!("Tokens column is not UInt32Array"))?;
        let tokens = if tokens_col.is_null(row) {
            None
        } else {
            Some(tokens_col.value(row) as u32)
        };

        let importance_arc = col("importance")?;
        let importance_col = importance_arc.as_ref().as_any().downcast_ref::<arrow_array::Float32Array>()
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

    /// Returns the most recent `limit` entries by time (Lance-only, not MemoryStore trait).
    ///
    /// Full table scan, then sort by `metadata.timestamp` descending in memory and take first `limit`.
    /// External: reads LanceDB table only, no network.
    ///
    /// # Arguments
    ///
    /// * `limit` - Max number of entries to return; 0 returns empty vec.
    ///
    /// # Returns
    ///
    /// List of `MemoryEntry` from newest to oldest.
    pub async fn list_recent(&self, limit: usize) -> Result<Vec<MemoryEntry>> {
        if limit == 0 {
            info!(limit = 0, "list_recent: limit is 0, returning empty");
            return Ok(Vec::new());
        }

        info!(limit = limit, "Querying Lance vector store for list_recent");
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let results = table
            .query()
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;

        let batches = results
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| anyhow!("Failed to collect results: {}", e))?;

        let mut entries = Vec::new();
        for batch in batches {
            for row in 0..batch.num_rows() {
                let entry = self.batch_to_entry(&batch, row)?;
                entries.push(entry);
            }
        }

        entries.sort_by(|a, b| b.metadata.timestamp.cmp(&a.metadata.timestamp));
        entries.truncate(limit);

        info!(
            requested = limit,
            returned = entries.len(),
            "Lance vector store list_recent returned"
        );
        Ok(entries)
    }

    /// Escapes a string for safe use in a Lance SQL predicate (e.g. only_if).
    /// Single quotes are doubled so values cannot break the predicate.
    fn escape_sql_string(s: &str) -> String {
        s.replace('\'', "''")
    }

    /// Converts Lance _distance to similarity score (higher = more similar).
    /// For Cosine distance: Lance typically returns 1 - cos_sim, so similarity = 1.0 - distance.
    /// If _distance column is missing, returns 1.0.
    fn distance_to_similarity(batch: &RecordBatch, distance_col_idx: Option<usize>, row: usize) -> f32 {
        let Some(idx) = distance_col_idx else {
            return 1.0;
        };
        let col = match batch.column(idx).as_any().downcast_ref::<Float32Array>() {
            Some(a) => a,
            None => return 1.0,
        };
        let distance = if col.is_null(row) {
            0.0
        } else {
            col.value(row)
        };
        // Cosine distance in Lance: smaller = more similar; often distance = 1 - cosine_sim.
        (1.0 - distance).max(0.0).min(1.0)
    }
}

#[async_trait]
impl MemoryStore for LanceVectorStore {
    async fn add(&self, entry: MemoryEntry) -> Result<()> {
        if entry.embedding.is_some() {
            info!(
                id = %entry.id,
                dimension = entry.embedding.as_ref().map(|e| e.len()).unwrap_or(0),
                "step: embedding Lance write vector"
            );
        }
        info!(
            id = %entry.id,
            user_id = ?entry.metadata.user_id,
            conversation_id = ?entry.metadata.conversation_id,
            role = ?entry.metadata.role,
            has_embedding = entry.embedding.is_some(),
            "Writing entry to Lance vector store"
        );

        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let batch = self.entry_to_batch(&entry)?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(reader)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to add entry: {}", e))?;

        info!(
            id = %entry.id,
            user_id = ?entry.metadata.user_id,
            conversation_id = ?entry.metadata.conversation_id,
            "Entry written to Lance vector store"
        );
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>> {
        info!(id = %id, "Querying Lance vector store by id");
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .execute()
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
                    info!(id = %id, found = true, "Lance vector store get returned");
                    return Ok(Some(entry));
                }
            }
        }

        info!(id = %id, found = false, "Lance vector store get returned");
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
            .execute()
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
        info!(user_id = %user_id, "Querying Lance vector store by user");
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .execute()
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
        entries.sort_by(|a, b| a.metadata.timestamp.cmp(&b.metadata.timestamp));

        info!(
            user_id = %user_id,
            count = entries.len(),
            "Lance vector store search_by_user returned"
        );
        Ok(entries)
    }

    async fn search_by_conversation(&self, conversation_id: &str) -> Result<Vec<MemoryEntry>> {
        info!(conversation_id = %conversation_id, "Querying Lance vector store by conversation");
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        let filter = format!("conversation_id = '{}'", Self::escape_sql_string(conversation_id));
        let results = table
            .query()
            .only_if(filter)
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
                entries.push(entry);
            }
        }
        entries.sort_by(|a, b| a.metadata.timestamp.cmp(&b.metadata.timestamp));

        info!(
            conversation_id = %conversation_id,
            count = entries.len(),
            "Lance vector store search_by_conversation returned"
        );
        Ok(entries)
    }

    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        user_id: Option<&str>,
        conversation_id: Option<&str>,
    ) -> Result<Vec<(f32, MemoryEntry)>> {
        info!(
            dimension = query_embedding.len(),
            limit = limit,
            user_id = ?user_id,
            conversation_id = ?conversation_id,
            "step: embedding Lance semantic search"
        );
        info!(
            embedding_len = query_embedding.len(),
            expected_dim = self.config.embedding_dim,
            limit = limit,
            "Querying Lance vector store semantic_search"
        );
        let db = self.db.read().await;
        let table = db
            .open_table(&self.config.table_name)
            .execute()
            .await
            .map_err(|e| {
                error!(error = %e, "Lance semantic_search: failed to open table");
                anyhow!("Failed to open table: {}", e)
            })?;

        // Push conversation_id and user_id to Lance so we only need limit (no over-fetch).
        let mut predicate_parts = Vec::new();
        if let Some(u) = user_id {
            predicate_parts.push(format!("user_id = '{}'", Self::escape_sql_string(u)));
        }
        if let Some(c) = conversation_id {
            predicate_parts.push(format!("conversation_id = '{}'", Self::escape_sql_string(c)));
        }
        let predicate = predicate_parts.join(" AND ");
        let filter_pushed_down = !predicate.is_empty();

        let mut vector_query = table
            .query()
            .nearest_to(query_embedding)
            .map_err(|e| {
                error!(
                    error = %e,
                    query_dim = query_embedding.len(),
                    expected_dim = self.config.embedding_dim,
                    "Lance semantic_search: failed to create vector query (dimension mismatch?)"
                );
                anyhow!(
                    "Failed to create vector query (query_dim={}, expected_dim={}): {}",
                    query_embedding.len(),
                    self.config.embedding_dim,
                    e
                )
            })?;

        if filter_pushed_down {
            vector_query = vector_query.only_if(predicate);
        }
        if self.config.use_exact_search {
            vector_query = vector_query.bypass_vector_index();
        }
        if let Some(rf) = self.config.refine_factor {
            vector_query = vector_query.refine_factor(rf);
        }
        if let Some(np) = self.config.nprobes {
            vector_query = vector_query.nprobes(np);
        }

        let results = vector_query
            .limit(limit)
            .execute()
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    query_dim = query_embedding.len(),
                    expected_dim = self.config.embedding_dim,
                    "Lance semantic_search: failed to execute vector search"
                );
                anyhow!(
                    "Failed to execute vector search (query_dim={}, expected_dim={}): {}",
                    query_embedding.len(),
                    self.config.embedding_dim,
                    e
                )
            })?;

        let results = results
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| {
                error!(error = %e, "Lance semantic_search: failed to collect results");
                anyhow!("Failed to collect results: {}", e)
            })?;

        let mut scored_entries: Vec<(f32, MemoryEntry)> = Vec::new();
        for batch in results {
            let distance_col_idx = batch.schema().index_of("_distance").ok();
            for row in 0..batch.num_rows() {
                let entry = self.batch_to_entry(&batch, row)?;
                if filter_pushed_down {
                    scored_entries.push((
                        Self::distance_to_similarity(&batch, distance_col_idx, row),
                        entry,
                    ));
                } else {
                    let match_user = user_id
                        .map(|u| entry.metadata.user_id.as_deref() == Some(u))
                        .unwrap_or(true);
                    let match_conv = conversation_id
                        .map(|c| entry.metadata.conversation_id.as_deref() == Some(c))
                        .unwrap_or(true);
                    if match_user && match_conv {
                        let score = Self::distance_to_similarity(&batch, distance_col_idx, row);
                        scored_entries.push((score, entry));
                    }
                }
            }
        }
        scored_entries.truncate(limit);

        info!(
            limit = limit,
            count = scored_entries.len(),
            "step: embedding Lance semantic search done"
        );
        info!(
            limit = limit,
            count = scored_entries.len(),
            "Lance vector store semantic_search returned"
        );
        Ok(scored_entries)
    }
}
