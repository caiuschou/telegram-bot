use memory::{MemoryEntry, MemoryMetadata, MemoryRole};

#[test]
fn test_memory_entry_creation() {
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: Some("conv456".to_string()),
        role: MemoryRole::User,
        timestamp: chrono::Utc::now(),
        tokens: Some(10),
        importance: Some(0.8),
    };

    let entry = MemoryEntry::new("Hello world".to_string(), metadata.clone());

    assert_eq!(entry.content, "Hello world");
    assert_eq!(entry.metadata.user_id, Some("user123".to_string()));
    assert_eq!(entry.metadata.conversation_id, Some("conv456".to_string()));
    assert_eq!(entry.metadata.role, MemoryRole::User);
    assert!(entry.embedding.is_none());
}

#[test]
fn test_memory_role_serialization() {
    let role = MemoryRole::User;
    let serialized = serde_json::to_string(&role).unwrap();
    assert_eq!(serialized, "\"User\"");

    let deserialized: MemoryRole = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, MemoryRole::User);
}

#[test]
fn test_memory_metadata() {
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: None,
        role: MemoryRole::Assistant,
        timestamp: chrono::Utc::now(),
        tokens: Some(20),
        importance: Some(0.5),
    };

    assert_eq!(metadata.user_id.as_deref(), Some("user123"));
    assert!(metadata.conversation_id.is_none());
    assert_eq!(metadata.role, MemoryRole::Assistant);
    assert_eq!(metadata.tokens, Some(20));
    assert_eq!(metadata.importance, Some(0.5));
}

#[test]
fn test_memory_entry_serialization() {
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: Some("conv456".to_string()),
        role: MemoryRole::User,
        timestamp: chrono::Utc::now(),
        tokens: Some(10),
        importance: Some(0.8),
    };

    let mut entry = MemoryEntry::new("Test content".to_string(), metadata);
    entry.embedding = Some(vec![0.1, 0.2, 0.3]);

    let serialized = serde_json::to_string(&entry).unwrap();
    let deserialized: MemoryEntry = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.content, "Test content");
    assert_eq!(deserialized.metadata.user_id, Some("user123".to_string()));
    assert_eq!(deserialized.metadata.role, MemoryRole::User);
    assert_eq!(deserialized.embedding, Some(vec![0.1, 0.2, 0.3]));
}
