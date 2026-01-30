//! MessageRecord → MemoryEntry 转换
//!
//! 将 SQLite 消息记录转换为向量库使用的 MemoryEntry。
//! 外部依赖：storage::MessageRecord、memory::MemoryEntry。

use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
use storage::MessageRecord;
use uuid::Uuid;

/// 将 MessageRecord 转换为 MemoryEntry
///
/// # 字段映射
///
/// - id: 保留原始 UUID
/// - content: message.content
/// - embedding: None（后续由 EmbeddingService 填充）
/// - metadata.user_id: message.user_id (转为 String)
/// - metadata.conversation_id: message.chat_id (转为 String)
/// - metadata.role: 根据 direction 判断
///   - "received" → MemoryRole::User
///   - "sent" → MemoryRole::Assistant
///   - 其他 → MemoryRole::User (默认)
/// - metadata.timestamp: message.created_at
/// - metadata.tokens: None
/// - metadata.importance: None
///
/// # 参数
///
/// * `msg` - SQLite 消息记录
///
/// # 返回
///
/// 返回转换后的 MemoryEntry
pub(crate) fn convert(msg: &MessageRecord) -> MemoryEntry {
    let role = match msg.direction.as_str() {
        "received" => MemoryRole::User,
        "sent" => MemoryRole::Assistant,
        _ => MemoryRole::User,
    };

    MemoryEntry {
        id: Uuid::parse_str(&msg.id).unwrap_or_else(|_| Uuid::new_v4()),
        content: msg.content.clone(),
        embedding: None,
        metadata: MemoryMetadata {
            user_id: Some(msg.user_id.to_string()),
            conversation_id: Some(msg.chat_id.to_string()),
            role,
            timestamp: msg.created_at,
            tokens: None,
            importance: None,
        },
    }
}
