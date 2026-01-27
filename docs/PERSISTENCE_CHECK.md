# 消息数据持久化功能检查报告

## 概述

检查了 Telegram Bot 的消息数据持久化功能，该功能使用 SQLite 数据库存储所有发送和接收的消息。

## 数据库设计

### 表结构 (messages)

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | TEXT | 消息唯一标识符 (UUID) |
| user_id | INTEGER | Telegram 用户 ID |
| chat_id | INTEGER | Telegram 聊天 ID |
| username | TEXT | Telegram 用户名 (可选) |
| first_name | TEXT | 用户名 (可选) |
| last_name | TEXT | 姓氏 (可选) |
| message_type | TEXT | 消息类型 (text, command 等) |
| content | TEXT | 消息内容 |
| direction | TEXT | 方向 (received/sent) |
| created_at | TEXT | 创建时间 (UTC) |

### 索引

以下字段已创建索引以提高查询性能：
- user_id
- chat_id
- created_at
- direction
- message_type

## 功能模块

### 1. MessageStorage 结构

核心存储管理器，使用 `SqlitePool` 管理数据库连接池。

### 2. 数据库初始化

- 自动创建数据库文件（如果不存在）
- 创建 messages 表
- 创建必要的索引

### 3. 消息保存

```rust
pub async fn save_message(&self, message: &MessageRecord) -> Result<(), sqlx::Error>
```

- 将消息记录插入数据库
- 记录操作日志

### 4. 消息查询

#### 4.1 通用查询
```rust
pub async fn get_messages(&self, query: &MessageQuery) -> Result<Vec<MessageRecord>, sqlx::Error>
```

支持按以下条件查询：
- user_id
- chat_id
- message_type
- direction
- start_date
- end_date
- limit

#### 4.2 按用户查询
```rust
pub async fn get_messages_by_user(&self, user_id: i64, limit: Option<i64>) -> Result<Vec<MessageRecord>, sqlx::Error>
```

#### 4.3 按聊天查询
```rust
pub async fn get_messages_by_chat(&self, chat_id: i64, limit: Option<i64>) -> Result<Vec<MessageRecord>, sqlx::Error>
```

### 5. 统计信息

```rust
pub async fn get_stats(&self) -> Result<MessageStats, sqlx::Error>
```

返回以下统计信息：
- total_messages: 总消息数
- sent_messages: 发送消息数
- received_messages: 接收消息数
- unique_users: 唯一用户数
- unique_chats: 唯一聊天数
- first_message: 第一条消息时间
- last_message: 最后一条消息时间

### 6. 搜索功能

```rust
pub async fn search_messages(&self, keyword: &str, limit: Option<i64>) -> Result<Vec<MessageRecord>, sqlx::Error>
```

使用 `LIKE` 查询搜索消息内容。

### 7. 清理功能

#### 7.1 删除用户消息
```rust
pub async fn delete_messages_by_user(&self, user_id: i64) -> Result<u64, sqlx::Error>
```

#### 7.2 清理旧消息
```rust
pub async fn cleanup_old_messages(&self, days: i32) -> Result<u64, sqlx::Error>
```

删除指定天数之前的所有消息。

## 自动清理任务

在 `telegram-bot` 中实现了后台任务，每天自动清理30天前的旧消息：

```rust
async fn cleanup_task(storage: Arc<MessageStorage>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400));

    loop {
        interval.tick().await;

        if let Err(e) = storage.cleanup_old_messages(30).await {
            log::error!("Cleanup failed: {}", e);
        }
    }
}
```

## 数据库配置

通过环境变量 `DATABASE_URL` 配置数据库连接：

```bash
# 默认值
DATABASE_URL=file:./telegram_bot.db

# 自定义路径
DATABASE_URL=file:/path/to/database.db

# 内存数据库（测试用）
DATABASE_URL=:memory:
```

## 使用示例

### 保存消息

```rust
let record = MessageRecord::new(
    user_id,
    chat_id,
    username,
    first_name,
    last_name,
    "text".to_string(),
    message_content.to_string(),
    "received".to_string(),
);

storage.save_message(&record).await?;
```

### 查询消息

```rust
let messages = storage.get_messages_by_user(user_id, Some(10)).await?;
```

### 获取统计

```rust
let stats = storage.get_stats().await?;
println!("Total messages: {}", stats.total_messages);
```

### 搜索消息

```rust
let results = storage.search_messages("keyword", Some(5)).await?;
```

## 功能验证清单

- [x] 数据库连接和初始化
- [x] 数据表创建（IF NOT EXISTS）
- [x] 索引创建以提高查询性能
- [x] 消息保存功能
- [x] 按用户查询消息
- [x] 按聊天查询消息
- [x] 通用条件查询
- [x] 统计信息获取
- [x] 消息搜索功能
- [x] 删除用户消息
- [x] 自动清理旧消息
- [x] 后台定时任务
- [x] 错误处理和日志记录

## 潜在改进建议

1. **连接池配置优化**：当前使用默认配置，可以根据实际负载调整连接池大小

2. **查询性能监控**：添加慢查询日志记录

3. **数据备份**：实现定期数据库备份功能

4. **数据归档**：将旧消息移动到归档表而非直接删除

5. **消息加密**：对敏感消息内容进行加密存储

6. **批量操作**：支持批量插入和批量删除以提高性能

7. **事务支持**：对于相关联的多个操作使用事务确保数据一致性

## 结论

消息数据持久化功能实现完整，包括：
- 完整的 CRUD 操作
- 灵活的查询接口
- 自动清理机制
- 统计和分析功能

所有功能都已正确集成到 Telegram Bot 主程序中，能够正常工作。
