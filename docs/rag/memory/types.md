# 核心类型

## MemoryRole

- **User** / **Assistant** / **System**

## MemoryMetadata

| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | Option<String> | 用户 ID |
| conversation_id | Option<String> | 会话 ID |
| role | MemoryRole | 角色 |
| timestamp | DateTime<Utc> | 时间戳 |
| tokens | Option<u32> | 预估 token |
| importance | Option<f32> | 重要性 0–1 |

## MemoryEntry

- id (Uuid)、content、embedding (Option<Vec<f32>>)、metadata。

定义见 memory 或 memory-core crate。
