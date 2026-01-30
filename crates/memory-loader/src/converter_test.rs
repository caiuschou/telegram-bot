//! convert 函数单元测试
//!
//! 测试 MessageRecord → MemoryEntry 的字段映射与 role 判断。

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use memory::MemoryRole;
    use storage::MessageRecord;

    use crate::converter::convert;

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
