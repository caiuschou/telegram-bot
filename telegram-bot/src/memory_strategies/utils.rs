//! Shared utilities for context strategies.

use crate::memory_core::{MemoryEntry, MemoryRole};

pub(crate) fn format_message(entry: &MemoryEntry) -> String {
    let role = match entry.metadata.role {
        MemoryRole::User => "User",
        MemoryRole::Assistant => "Assistant",
        MemoryRole::System => "System",
    };
    format!("{}: {}", role, entry.content)
}

pub(crate) fn extract_preferences(entries: &[MemoryEntry]) -> Vec<String> {
    let mut preferences = Vec::new();
    for entry in entries {
        let content = entry.content.to_lowercase();
        if content.contains("i like") || content.contains("i prefer") {
            if let Some(start) = content.find("i like") {
                preferences.push(content[start..].to_string());
            } else if let Some(start) = content.find("i prefer") {
                preferences.push(content[start..].to_string());
            }
        }
    }
    preferences
}
