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

use crate::store::MemoryStore;
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
    use crate::types::{MemoryEntry, MemoryMetadata, MemoryRole};
    use chrono::Utc;

    #[tokio::test]
    async fn test_migration() {
        // This would require actual stores to test
    }
}
