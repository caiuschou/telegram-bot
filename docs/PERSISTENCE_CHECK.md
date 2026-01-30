# 消息持久化功能检查报告

## 概述

Telegram Bot 使用 SQLite（messages 表）存储收发消息；实现位于 storage 与 telegram-bot 持久化中间件。

## 表结构 (messages)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | UUID |
| user_id, chat_id | INTEGER | Telegram ID |
| username, first_name, last_name | TEXT | 可选 |
| message_type | TEXT | text / command 等 |
| content | TEXT | 消息内容 |
| direction | TEXT | received / sent |
| created_at | TEXT | UTC |

索引：user_id、chat_id、created_at、direction、message_type。

## 功能要点

- 初始化：自动建表与索引。
- 保存：`save_message(MessageRecord)`。
- 查询：按 user_id、chat_id、message_type、direction、时间范围、limit；`get_messages_by_user`、`get_messages_by_chat`。
- 统计：`get_stats()`（总数、收发数、唯一用户/会话、首末消息时间）。
- 搜索：`search_messages(keyword, limit)`（LIKE）。
- 清理：`cleanup_old_messages(days)`；可配后台定时（如 30 天）。

## 配置

`DATABASE_URL`，默认 `file:./telegram_bot.db`；测试可用 `:memory:`。

## 验证清单

- [x] 建表与索引
- [x] 保存 / 按用户·会话查询 / 通用查询
- [x] 统计与搜索
- [x] 删除与定时清理
- [x] 错误与日志

结论：持久化功能完整，已集成到主程序。
