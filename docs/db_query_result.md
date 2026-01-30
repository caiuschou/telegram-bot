# 数据库查询结果说明

## 1. 项目内数据库文件

| 路径 | 表 | 说明 |
|------|----|------|
| `memory/memory.db`、`crates/memory-sqlite/memory.db` | memory_entries | 测试数据 |
| `telegram_bot.db` | messages | 消息持久化，真实对话 |

AI 的「Recent Message」在 `MEMORY_RECENT_USE_SQLITE=1` 时来自 **memory_entries**（`MEMORY_SQLITE_PATH`）；未配置时可能用 `./data/memory.db`（仓库内可能不存在）。

## 2. messages 表（消息持久化）

按会话、`created_at DESC` 查询；策略层会再按时间升序取最后 N 条发给 AI。字段：id、user_id、chat_id、username、first_name、last_name、message_type、content、direction、created_at。

## 3. 与 AI 请求的「Conversation (recent)」对照

- 喂给 AI 的 recent 来自 **memory_entries**（SQLite 或 Lance），不是 messages 表。
- messages 表可用来核对「同一对话内容与顺序」是否与请求一致；若需核对真正喂给 AI 的 recent，需在有 `./data/memory.db`（或当前 MEMORY_SQLITE_PATH）的环境查 memory_entries。

## 4. 自查 SQL 示例

```bash
# 消息持久化：按会话降序
sqlite3 telegram_bot.db "SELECT created_at, direction, message_type, content FROM messages WHERE chat_id = ? ORDER BY created_at DESC;"

# 最近消息用 SQLite：按 conversation 降序
sqlite3 ./data/memory.db "SELECT timestamp, role, content FROM memory_entries WHERE conversation_id = ? ORDER BY timestamp DESC;"
```
