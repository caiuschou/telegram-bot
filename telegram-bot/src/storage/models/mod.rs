//! Data models for storage (message records, queries, stats).
//!
//! Used by MessageRepository and callers of the storage API.

mod message_query;
mod message_record;
mod message_stats;

pub use message_query::MessageQuery;
pub use message_record::MessageRecord;
pub use message_stats::MessageStats;
