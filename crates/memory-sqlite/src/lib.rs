//! # SQLite Vector Store
//!
//! This module provides an SQLite-based implementation of the `MemoryStore` trait.
//!
//! ## SQLiteVectorStore
//!
//! Persistent storage using SQLite for memory entries and vector embeddings.
//!
//! **Advantages**:
//! - Persistent storage (data survives restarts)
//! - Good balance of performance and simplicity
//! - No external database required
//! - Easy to set up and maintain
//!
//! **Limitations**:
//! - Limited vector search performance for large datasets
//! - Not optimized for high-volume vector operations
//! - Single-file database (can become large)
//!
//! ## Example
//!
//! ```rust
//! use memory_sqlite::SQLiteVectorStore;
//! use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
//! use chrono::Utc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), anyhow::Error> {
//!     let store = SQLiteVectorStore::new("memory.db").await?;
//!
//!     let metadata = MemoryMetadata {
//!         user_id: Some("user123".to_string()),
//!         conversation_id: None,
//!         role: MemoryRole::User,
//!         timestamp: Utc::now(),
//!         tokens: None,
//!         importance: None,
//!     };
//!     let entry = MemoryEntry::new("Hello world".to_string(), metadata);
//!
//!     store.add(entry).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Database Schema
//!
//! The store uses the following table structure:
//!
//! ```sql
//! CREATE TABLE memory_entries (
//!     id TEXT PRIMARY KEY,
//!     content TEXT NOT NULL,
//!     user_id TEXT,
//!     conversation_id TEXT,
//!     role TEXT NOT NULL,
//!     timestamp TEXT NOT NULL,
//!     tokens INTEGER,
//!     importance REAL,
//!     embedding BLOB
//! );
//! ```

use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use chrono::{DateTime, Utc};
use std::str::FromStr;
use uuid::Uuid;

/// SQLite-based vector store for persistent memory storage.
#[derive(Clone)]
pub struct SQLiteVectorStore {
    pool: SqlitePool,
}

impl SQLiteVectorStore {
    /// Creates a new SQLite vector store with the specified database file.
    ///
    /// # Arguments
    ///
    /// * `database_url` - Path to the SQLite database file (e.g., "memory.db").
    ///
    /// # Returns
    ///
    /// A new `SQLiteVectorStore` instance with initialized database schema.
    ///
    /// # Errors
    ///
    /// Returns an error if database connection or schema initialization fails.
    pub async fn new(database_url: &str) -> Result<Self, anyhow::Error> {
        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(database_url);

        let pool = SqlitePool::connect_with(options).await?;

        let store = Self { pool };
        store.init_schema().await?;

        Ok(store)
    }

    /// Initializes the database schema.
    ///
    /// Creates the necessary tables and indexes for storing memory entries and
    /// their vector embeddings. This method is called automatically during store
    /// initialization.
    ///
    /// # Database Structure
    ///
    /// Creates a `memory_entries` table with the following columns:
    /// - id (TEXT PRIMARY KEY): UUID of the memory entry
    /// - content (TEXT NOT NULL): Message content
    /// - user_id (TEXT): Optional user identifier
    /// - conversation_id (TEXT): Optional conversation identifier
    /// - role (TEXT NOT NULL): Message role (User/Assistant/System)
    /// - timestamp (TEXT NOT NULL): ISO 8601 timestamp
    /// - tokens (INTEGER): Optional token count
    /// - importance (REAL): Optional importance score
    /// - embedding (BLOB): Vector embedding as binary data
    ///
    /// # Indexes Created
    ///
    /// - idx_user_id: For fast user-based queries
    /// - idx_conversation_id: For fast conversation-based queries
    /// - idx_timestamp: For time-based sorting
    ///
    /// # External Interactions
    ///
    /// - **SQLite**: Executes DDL commands to create schema
    async fn init_schema(&self) -> Result<(), anyhow::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memory_entries (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                user_id TEXT,
                conversation_id TEXT,
                role TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tokens INTEGER,
                importance REAL,
                embedding BLOB
            );

            CREATE INDEX IF NOT EXISTS idx_user_id ON memory_entries(user_id);
            CREATE INDEX IF NOT EXISTS idx_conversation_id ON memory_entries(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_timestamp ON memory_entries(timestamp);
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Converts a database row to a MemoryEntry.
    ///
    /// Deserializes data from SQLite row format into the in-memory MemoryEntry structure.
    /// Handles type conversions for complex fields like UUIDs, timestamps, and binary
    /// vector embeddings.
    ///
    /// # Conversion Process
    ///
    /// 1. Extracts and parses UUID string to Uuid type
    /// 2. Converts role string ("User"/"Assistant"/"System") to MemoryRole enum
    /// 3. Parses ISO 8601 timestamp string to DateTime<Utc>
    /// 4. Deserializes BLOB field to Vec<f32> for embeddings:
    ///    - Reads binary data in little-endian format (4 bytes per float)
    ///    - Converts each 4-byte chunk to f32 using from_le_bytes
    ///
    /// # External Interactions
    ///
    /// - **SQLite**: Reads BLOB data for embeddings stored in binary format
    /// - **chrono**: Parses timestamp strings from database
    ///
    /// # Error Handling
    ///
    /// Returns error for:
    /// - Invalid UUID format
    /// - Unknown role values
    /// - Invalid timestamp format
    /// - BLOB data length not divisible by 4 (invalid embedding data)
    fn row_to_entry(row: &sqlx::sqlite::SqliteRow) -> Result<MemoryEntry, sqlx::Error> {
        let id: String = row.try_get("id")?;
        let content: String = row.try_get("content")?;
        let user_id: Option<String> = row.try_get("user_id")?;
        let conversation_id: Option<String> = row.try_get("conversation_id")?;
        let role_str: String = row.try_get("role")?;
        let timestamp_str: String = row.try_get("timestamp")?;
        let tokens: Option<i64> = row.try_get("tokens")?;
        let importance: Option<f64> = row.try_get("importance")?;
        let embedding_blob: Option<Vec<u8>> = row.try_get("embedding")?;

        let id = Uuid::from_str(&id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let role = match role_str.as_str() {
            "User" => MemoryRole::User,
            "Assistant" => MemoryRole::Assistant,
            "System" => MemoryRole::System,
            _ => return Err(sqlx::Error::Decode(Box::new(
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid role")
            ))),
        };

        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
            .with_timezone(&Utc);

        let embedding = embedding_blob.map(|blob| {
            blob.chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        });

        let metadata = MemoryMetadata {
            user_id,
            conversation_id,
            role,
            timestamp,
            tokens: tokens.map(|t| t as u32),
            importance: importance.map(|i| i as f32),
        };

        Ok(MemoryEntry {
            id,
            content,
            embedding,
            metadata,
        })
    }

    /// Calculates cosine similarity between two vectors.
    ///
    /// Computes the cosine similarity metric, which measures the cosine of the angle
    /// between two vectors. This is a standard similarity metric for vector embeddings,
    /// ranging from -1 (opposite) to 1 (identical), with 0 indicating orthogonality.
    ///
    /// # Algorithm
    ///
    /// Similarity = (a · b) / (||a|| * ||b||)
    ///
    /// Where:
    /// - a · b = dot product (sum of element-wise products)
    /// - ||a|| = Euclidean norm (square root of sum of squares)
    ///
    /// # Special Cases
    ///
    /// - Empty vectors return 0.0 similarity
    /// - Zero vectors return 0.0 similarity (to avoid division by zero)
    ///
    /// # External Interactions
    ///
    /// - **Semantic Search**: Used to rank memory entries by relevance to query
    /// - **Vector Databases**: Standard similarity metric for embedding comparisons
    ///
    /// # Performance
    ///
    /// Time complexity: O(n) where n is vector dimensionality.
    /// Memory complexity: O(1) - only accumulators used.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

#[async_trait::async_trait]
impl MemoryStore for SQLiteVectorStore {
    /// Adds a new memory entry to the store.
    ///
    /// Persists a new memory entry to the SQLite database, including all metadata
    /// and optionally the vector embedding for semantic search capabilities.
    ///
    /// # External Interactions
    ///
    /// - **SQLite Database**: Executes INSERT statement to store entry in memory_entries table
    /// - **File System**: Data is written to the SQLite database file on disk
    /// - **Storage Persistence**: Entry survives application restarts
    ///
    /// # Data Transformation
    ///
    /// - UUID: Converted to string for TEXT storage
    /// - Timestamp: Converted to ISO 8601 string (RFC3339)
    /// - Role: Converted to string ("User"/"Assistant"/"System")
    /// - Tokens: Converted from u32 to i64
    /// - Importance: Converted from f32 to f64
    /// - Embedding: Serialized to binary BLOB (little-endian, 4 bytes per float)
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let role_str = match entry.metadata.role {
            MemoryRole::User => "User",
            MemoryRole::Assistant => "Assistant",
            MemoryRole::System => "System",
        };

        let timestamp_str = entry.metadata.timestamp.to_rfc3339();

        let embedding_blob: Option<Vec<u8>> = entry.embedding.map(|embedding| {
            embedding
                .iter()
                .flat_map(|f| f.to_le_bytes().to_vec())
                .collect()
        });

        sqlx::query(
            r#"
            INSERT INTO memory_entries (
                id, content, user_id, conversation_id, role, timestamp,
                tokens, importance, embedding
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#
        )
        .bind(entry.id.to_string())
        .bind(&entry.content)
        .bind(&entry.metadata.user_id)
        .bind(&entry.metadata.conversation_id)
        .bind(role_str)
        .bind(&timestamp_str)
        .bind(entry.metadata.tokens.map(|t| t as i64))
        .bind(entry.metadata.importance.map(|i| i as f64))
        .bind(embedding_blob)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieves a memory entry by its UUID. Returns `None` if not found.
    ///
    /// Queries the database for a specific memory entry using its unique identifier.
    ///
    /// # External Interactions
    ///
    /// - **SQLite Database**: Executes SELECT query with WHERE id = ? condition
    /// - **Storage**: Reads from persistent SQLite database file
    ///
    /// # Performance
    ///
    /// - Uses indexed primary key lookup (O(log n) in B-tree)
    /// - Fast retrieval due to PRIMARY KEY index on id column
    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        let row = sqlx::query("SELECT * FROM memory_entries WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(Some(Self::row_to_entry(&r)?)),
            None => Ok(None),
        }
    }

    /// Updates an existing memory entry.
    ///
    /// Modifies all fields of an existing memory entry in the database. This is
    /// a full replacement operation where all field values are overwritten.
    ///
    /// # External Interactions
    ///
    /// - **SQLite Database**: Executes UPDATE statement with WHERE id = ? condition
    /// - **File System**: Writes updated data to database file on disk
    /// - **Storage Persistence**: Changes are immediately persisted
    ///
    /// # Data Transformation
    ///
    /// Same transformation rules as add() method:
    /// - UUID, timestamp, role, tokens, importance, embedding all converted to storage format
    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let role_str = match entry.metadata.role {
            MemoryRole::User => "User",
            MemoryRole::Assistant => "Assistant",
            MemoryRole::System => "System",
        };

        let timestamp_str = entry.metadata.timestamp.to_rfc3339();

        let embedding_blob: Option<Vec<u8>> = entry.embedding.map(|embedding| {
            embedding
                .iter()
                .flat_map(|f| f.to_le_bytes().to_vec())
                .collect()
        });

        sqlx::query(
            r#"
            UPDATE memory_entries SET
                content = ?1,
                user_id = ?2,
                conversation_id = ?3,
                role = ?4,
                timestamp = ?5,
                tokens = ?6,
                importance = ?7,
                embedding = ?8
            WHERE id = ?9
            "#
        )
        .bind(&entry.content)
        .bind(&entry.metadata.user_id)
        .bind(&entry.metadata.conversation_id)
        .bind(role_str)
        .bind(&timestamp_str)
        .bind(entry.metadata.tokens.map(|t| t as i64))
        .bind(entry.metadata.importance.map(|i| i as f64))
        .bind(embedding_blob)
        .bind(entry.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes a memory entry by its UUID.
    ///
    /// Removes a memory entry permanently from the database. This operation is
    /// irreversible and will also delete any associated vector embedding.
    ///
    /// # External Interactions
    ///
    /// - **SQLite Database**: Executes DELETE statement with WHERE id = ? condition
    /// - **File System**: Writes deletion to database file (may trigger page cleanup)
    /// - **Storage Persistence**: Entry is permanently removed, cannot be recovered
    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM memory_entries WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Retrieves all memory entries for a specific user.
    ///
    /// Queries the database for all entries belonging to a given user, ordered
    /// by timestamp in descending order (most recent first).
    ///
    /// # External Interactions
    ///
    /// - **SQLite Database**: Executes SELECT query with WHERE user_id = ? condition
    /// - **Index Usage**: Utilizes idx_user_id index for efficient filtering
    /// - **Storage**: Reads multiple rows from database file
    ///
    /// # Performance
    ///
    /// - O(k) where k is number of entries for the user
    /// - Uses indexed lookup for user_id column
    /// - Results sorted by timestamp during query execution
    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let rows = sqlx::query("SELECT * FROM memory_entries WHERE user_id = ?1 ORDER BY timestamp DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(Self::row_to_entry(&row)?);
        }

        Ok(entries)
    }

    /// Retrieves all memory entries for a specific conversation.
    ///
    /// Queries the database for all entries belonging to a given conversation,
    /// ordered by timestamp in descending order (most recent first).
    ///
    /// # External Interactions
    ///
    /// - **SQLite Database**: Executes SELECT query with WHERE conversation_id = ? condition
    /// - **Index Usage**: Utilizes idx_conversation_id index for efficient filtering
    /// - **Storage**: Reads multiple rows from database file
    ///
    /// # Performance
    ///
    /// - O(k) where k is number of entries in the conversation
    /// - Uses indexed lookup for conversation_id column
    /// - Results sorted by timestamp during query execution
    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let rows = sqlx::query("SELECT * FROM memory_entries WHERE conversation_id = ?1 ORDER BY timestamp DESC")
            .bind(conversation_id)
            .fetch_all(&self.pool)
            .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(Self::row_to_entry(&row)?);
        }

        Ok(entries)
    }

    /// Performs semantic search using vector embeddings.
    ///
    /// Returns the top `limit` most similar entries based on cosine similarity.
    /// This method finds memory entries that are semantically similar to the query
    /// by comparing their vector embeddings.
    ///
    /// # Algorithm
    ///
    /// 1. Queries SQLite for all entries that have embeddings (WHERE embedding IS NOT NULL)
    /// 2. For each entry, calculates cosine similarity with query_embedding
    /// 3. Sorts entries by similarity score in descending order
    /// 4. Returns top `limit` entries with highest similarity
    ///
    /// # External Interactions
    ///
    /// - **SQLite Database**: Reads all embedding vectors from storage
    /// - **Embedding Services**: Query embedding typically comes from OpenAI embedding API
    /// - **Memory Operations**: Loads all vectors into memory for similarity calculation
    ///
    /// # Performance Characteristics
    ///
    /// - Time complexity: O(n * d) where n is number of entries, d is vector dimension
    /// - Memory complexity: O(n * d) - loads all embeddings into RAM
    /// - Not scalable for large datasets (>100K entries)
    ///
    /// # Limitations
    ///
    /// Note: This retrieves all entries with embeddings and calculates similarity in-memory.
    /// For large datasets, consider using a specialized vector database like Lance.
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - Vector embedding of the search query
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Vector of memory entries sorted by similarity (highest first).
    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let rows = sqlx::query("SELECT * FROM memory_entries WHERE embedding IS NOT NULL")
            .fetch_all(&self.pool)
            .await?;

        let mut similarities: Vec<(f32, MemoryEntry)> = Vec::new();
        for row in rows {
            let entry = Self::row_to_entry(&row)?;
            if let Some(embedding) = &entry.embedding {
                let similarity = Self::cosine_similarity(query_embedding, embedding);
                similarities.push((similarity, entry));
            }
        }

        similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<MemoryEntry> = similarities
            .into_iter()
            .take(limit)
            .map(|(_, entry)| entry)
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_store() -> SQLiteVectorStore {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let path = db_path.to_str().unwrap().to_string();

        let store = SQLiteVectorStore::new(&path).await.unwrap();

        std::mem::forget(temp_dir);

        store
    }

    fn create_test_entry(content: &str, user_id: &str) -> MemoryEntry {
        let metadata = MemoryMetadata {
            user_id: Some(user_id.to_string()),
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        MemoryEntry::new(content.to_string(), metadata)
    }

    #[tokio::test]
    async fn test_add_and_get() {
        let store = create_test_store().await;
        let entry = create_test_entry("Test content", "user123");

        store.add(entry.clone()).await.unwrap();

        let found = store.get(entry.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Test content");
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = create_test_store().await;
        let found = store.get(Uuid::new_v4()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let store = create_test_store().await;
        let mut entry = create_test_entry("Original", "user123");
        store.add(entry.clone()).await.unwrap();

        entry.content = "Updated".to_string();
        store.update(entry.clone()).await.unwrap();

        let found = store.get(entry.id).await.unwrap().unwrap();
        assert_eq!(found.content, "Updated");
    }

    #[tokio::test]
    async fn test_delete() {
        let store = create_test_store().await;
        let entry = create_test_entry("Test", "user123");
        store.add(entry.clone()).await.unwrap();

        store.delete(entry.id).await.unwrap();

        let found = store.get(entry.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_search_by_user() {
        let store = create_test_store().await;

        let entry1 = create_test_entry("Hello", "user123");
        let entry2 = create_test_entry("World", "user123");
        let entry3 = create_test_entry("Other", "user456");

        store.add(entry1).await.unwrap();
        store.add(entry2).await.unwrap();
        store.add(entry3).await.unwrap();

        let results = store.search_by_user("user123").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_by_conversation() {
        let store = create_test_store().await;

        let metadata1 = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: Some("conv1".to_string()),
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let entry1 = MemoryEntry::new("Hello".to_string(), metadata1);

        let metadata2 = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: Some("conv2".to_string()),
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let entry2 = MemoryEntry::new("World".to_string(), metadata2);

        store.add(entry1).await.unwrap();
        store.add(entry2).await.unwrap();

        let results = store.search_by_conversation("conv1").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let store = create_test_store().await;

        let metadata1 = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let mut entry1 = MemoryEntry::new("Hello world".to_string(), metadata1);
        entry1.embedding = Some(vec![1.0, 0.0, 0.0]);

        let metadata2 = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let mut entry2 = MemoryEntry::new("Goodbye world".to_string(), metadata2);
        entry2.embedding = Some(vec![0.0, 1.0, 0.0]);

        let metadata3 = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let mut entry3 = MemoryEntry::new("Hello there".to_string(), metadata3);
        entry3.embedding = Some(vec![0.9, 0.1, 0.0]);

        store.add(entry1).await.unwrap();
        store.add(entry2).await.unwrap();
        store.add(entry3).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.semantic_search(&query, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content, "Hello world");
    }
}
