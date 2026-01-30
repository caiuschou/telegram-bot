//! Unit tests for [`crate::converter::convert`].
//!
//! Covers field mapping from [`storage::MessageRecord`] to [`memory::MemoryEntry`] and role derivation from `direction`.

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use memory::MemoryRole;
    use storage::MessageRecord;

    use crate::converter::convert;

    /// **Test: Received message maps to User role and correct content/metadata.**
    ///
    /// **Setup:** A `MessageRecord` with `direction = "received"`.
    /// **Expected:** `content` and ids match; `metadata.role` is `MemoryRole::User`; `embedding` is `None`.
    #[test]
    fn test_convert_received_message() {
        let msg = MessageRecord {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            user_id: 123456,
            chat_id: 789012,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            message_type: "text".to_string(),
            content: "Hello world".to_string(),
            direction: "received".to_string(),
            created_at: Utc::now(),
        };

        let entry = convert(&msg);

        assert_eq!(entry.content, "Hello world");
        assert_eq!(entry.metadata.user_id, Some("123456".to_string()));
        assert_eq!(entry.metadata.conversation_id, Some("789012".to_string()));
        assert_eq!(entry.metadata.role, MemoryRole::User);
        assert!(entry.embedding.is_none());
    }

    /// **Test: Sent message maps to Assistant role.**
    ///
    /// **Setup:** A `MessageRecord` with `direction = "sent"`.
    /// **Expected:** `metadata.role` is `MemoryRole::Assistant`; content preserved.
    #[test]
    fn test_convert_sent_message() {
        let msg = MessageRecord {
            id: "550e8400-e29b-41d4-a716-446655440001".to_string(),
            user_id: 123456,
            chat_id: 789012,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            message_type: "text".to_string(),
            content: "Bot response".to_string(),
            direction: "sent".to_string(),
            created_at: Utc::now(),
        };

        let entry = convert(&msg);

        assert_eq!(entry.content, "Bot response");
        assert_eq!(entry.metadata.role, MemoryRole::Assistant);
    }

    /// **Test: Unknown direction defaults to User role.**
    ///
    /// **Setup:** A `MessageRecord` with `direction = "unknown"`.
    /// **Expected:** `metadata.role` is `MemoryRole::User`.
    #[test]
    fn test_convert_unknown_direction() {
        let msg = MessageRecord {
            id: "550e8400-e29b-41d4-a716-446655440002".to_string(),
            user_id: 123456,
            chat_id: 789012,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            message_type: "text".to_string(),
            content: "Unknown direction".to_string(),
            direction: "unknown".to_string(),
            created_at: Utc::now(),
        };

        let entry = convert(&msg);

        assert_eq!(entry.metadata.role, MemoryRole::User);
    }

    /// **Test: Invalid UUID in message id is replaced by a new v4 UUID.**
    ///
    /// **Setup:** A `MessageRecord` with `id = "invalid-uuid"`.
    /// **Expected:** `entry.id` is not the literal "invalid-uuid" (converter generates new UUID).
    #[test]
    fn test_convert_invalid_uuid() {
        let msg = MessageRecord {
            id: "invalid-uuid".to_string(),
            user_id: 123456,
            chat_id: 789012,
            username: None,
            first_name: None,
            last_name: None,
            message_type: "text".to_string(),
            content: "Test".to_string(),
            direction: "received".to_string(),
            created_at: Utc::now(),
        };

        let entry = convert(&msg);

        assert_ne!(entry.id.to_string(), "invalid-uuid");
    }
}
