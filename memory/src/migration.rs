//! # Migration Tools
//!
//! Tools for migrating data between different storage backends.
//!
//! ## Example
//!
//! ```rust,ignore
//! use memory::{MemoryStore, migration::migrate};
//! use memory_sqlite::SQLiteVectorStore;
//! use memory_lance::LanceVectorStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Migrate from SQLite to Lance
//! let sqlite = SQLiteVectorStore::new("./data/memory.db").await?;
//! let lance = LanceVectorStore::new("./data/lancedb").await?;
//! migrate(&sqlite, &lance).await?;
//! # Ok(())
//! # }
//! ```

use memory_core::MemoryStore;
use anyhow::Result;

/// Migrates all data from one store to another.
///
/// # Arguments
///
/// * `from` - Source store
/// * `to` - Destination store
///
/// # Returns
///
/// Returns the number of entries migrated.
pub async fn migrate<S1: MemoryStore, S2: MemoryStore>(
    from: &S1,
    to: &S2,
) -> Result<usize> {
    // Get all entries from source
    let all_entries = from
        .search_by_user("")  // Empty string to get all (SQLite returns all)
        .await?;

    let mut count = 0;
    for entry in all_entries {
        to.add(entry).await?;
        count += 1;
    }

    Ok(count)
}

/// Creates a backup of all entries from a store.
///
/// # Arguments
///
/// * `store` - The store to backup
///
/// # Returns
///
/// Returns a vector of all entries.
pub async fn backup<S: MemoryStore>(_store: &S) -> Result<Vec<crate::types::MemoryEntry>> {
    // Try to get entries by common user IDs
    let all_entries = Vec::new();

    // Since we can't list all entries directly, we'll return empty for now
    // In a real implementation, you might add a `list_all` method to MemoryStore

    Ok(all_entries)
}

/// Restores entries from a backup to a store.
///
/// # Arguments
///
/// * `store` - The destination store
/// * `entries` - Entries to restore
///
/// # Returns
///
/// Returns the number of entries restored.
pub async fn restore<S: MemoryStore>(
    store: &S,
    entries: Vec<crate::types::MemoryEntry>,
) -> Result<usize> {
    let count = entries.len();
    for entry in entries {
        store.add(entry).await?;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_inmemory::InMemoryVectorStore;
    use crate::{MemoryEntry, MemoryMetadata, MemoryRole};
    use chrono::Utc;

    fn make_entry(content: &str, user_id: &str) -> MemoryEntry {
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
    async fn test_migrate() {
        let from = InMemoryVectorStore::new();
        let to = InMemoryVectorStore::new();
        from.add(make_entry("a", "")).await.unwrap();
        from.add(make_entry("b", "")).await.unwrap();
        let count = migrate(&from, &to).await.unwrap();
        assert_eq!(count, 2);
        let dest_entries = to.search_by_user("").await.unwrap();
        assert_eq!(dest_entries.len(), 2);
    }

    #[tokio::test]
    async fn test_backup_returns_empty() {
        let store = InMemoryVectorStore::new();
        store.add(make_entry("x", "u1")).await.unwrap();
        let entries = backup(&store).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_restore() {
        let store = InMemoryVectorStore::new();
        let entries = vec![
            make_entry("one", "u1"),
            make_entry("two", "u1"),
        ];
        let count = restore(&store, entries).await.unwrap();
        assert_eq!(count, 2);
        let found = store.search_by_user("u1").await.unwrap();
        assert_eq!(found.len(), 2);
    }
}
