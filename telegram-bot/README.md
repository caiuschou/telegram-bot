# Telegram Bot

基于 Teloxide 的功能完整、类型安全的 Telegram Bot 开发框架，带有消息记录功能。

## 项目结构

```
telegram-bot/
├── src/
│   ├── main.rs          # 主程序
│   ├── logger.rs        # 日志系统
│   ├── models.rs        # 数据模型
│   ├── storage.rs       # 数据库存储
│   └── db_manager.rs    # 数据库管理工具
├── Cargo.toml           # 项目配置
├── .env.example         # 环境变量示例
├── logs/
│   └── telegram-bot.log   # Bot 日志文件
└── telegram_bot.db       # SQLite 数据库
```

## 功能特性

### 消息记录系统
- ✅ 所有消息自动保存到 SQLite 数据库
- ✅ 支持收发消息记录
- ✅ 用户信息和聊天 ID 记录
- ✅ 精确到毫秒的时间戳
- ✅ 多种查询方式

### 数据库功能
- ✅ SQLite 数据库（轻量级，无需额外安装）
- ✅ 自动创建数据库表和索引
- ✅ 支持多种查询方式
- ✅ 数据导出功能
- ✅ 定期清理任务

### Bot 命令
| 命令 | 功能 |
|------|------|
| `/start` | 开始对话 |
| `/help` | 显示帮助信息 |
| `/echo <text>` | 回复消息 |
| `/stats` | 显示统计信息 |
| `/history` | 查看历史消息 |
| `/search <keyword>` | 搜索消息 |

### 数据库管理工具
- `db-manager stats` - 显示数据库统计
- `db-manager history <user_id> [limit]` - 查看用户历史
- `db-manager search <keyword> [limit]` - 搜索消息
- `db-manager cleanup <days>` - 清理旧消息
- `db-manager export` - 导出所有消息到 JSON

## 快速开始

### 1. 配置环境变量

```bash
cd telegram-bot
cp .env.example .env
nano .env  # 编辑填入你的 BOT_TOKEN
```

### 2. 编译和运行

```bash
# 编译项目
cargo build --release

# 运行 Bot
export BOT_TOKEN=your_bot_token_here
./target/release/telegram-bot

# 查看日志
tail -f logs/telegram-bot.log
```

### 3. 运行数据库管理工具

```bash
# 显示统计信息
./target/release/db-manager stats

# 查看用户历史
./target/release/db-manager history 1234567890

# 搜索消息
./target/release/db-manager search "hello" 10

# 清理 30 天前的消息
./target/release/db-manager cleanup 30

# 导出所有消息
./target/release/db-manager export
```

## 数据库表结构

### messages 表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT (PRIMARY KEY) | 消息唯一 ID |
| user_id | INTEGER | 用户 ID |
| chat_id | INTEGER | 聊天 ID |
| username | TEXT | 用户名 |
| first_name | TEXT | 名字 |
| last_name | TEXT | 姓氏 |
| message_type | TEXT | 消息类型（text, command 等）|
| content | TEXT | 消息内容 |
| direction | TEXT | 方向（sent/received）|
| created_at | TEXT | 创建时间 |

### 索引

为优化查询性能，以下字段已建立索引：
- user_id
- chat_id
- created_at
- direction
- message_type

## 使用方法

### Bot 命令使用

1. **获取 Bot Token**
   - 在 Telegram 中搜索 @BotFather
   - 发送 `/newbot` 命令
   - 按照提示创建 Bot 并获取 Token

2. **运行 Bot**
   ```bash
   export BOT_TOKEN=your_bot_token_here
   ./target/release/telegram-bot
   ```

3. **查看日志**
   ```bash
   tail -f logs/telegram-bot.log
   grep ERROR logs/telegram-bot.log
   ```

4. **数据库管理**
   ```bash
   ./target/release/db-manager stats
   ./target/release/db-manager history 1234567890 10
   ./target/release/db-manager search "hello" 10
   ./target/release/db-manager export
   ```

## 高级功能

### 1. 消息查询
```rust
// 按用户查询消息
let messages = storage.get_messages_by_user(user_id, Some(10)).await;

// 按聊天查询消息
let messages = storage.get_messages_by_chat(chat_id, Some(20)).await;

// 按关键词搜索
let messages = storage.search_messages("hello", Some(10)).await;

// 查看统计信息
let stats = storage.get_stats().await;
```

### 2. 数据导出
```bash
# 导出所有消息到 JSON 文件
./target/release/db-manager export

# 输出文件示例：messages_export_20260123_141530.json
```

### 3. 自动清理
Bot 内置定期清理任务（默认每天一次），会自动删除 30 天前的消息。

### 4. 性能优化
- ✅ 合理使用索引
- ✅ 定期清理旧数据
- ✅ 使用连接池

## 开发指南

### 添加新的 Bot 命令

```rust
else if text == "/newcommand" {
    // 你的逻辑
}
```

### 添加数据库查询

```rust
async fn get_custom_messages(&self, query: MessageQuery) -> Result<Vec<MessageRecord>, sqlx::Error> {
    // 你的查询逻辑
}
```

## 文档

详细文档请查看 `docs/` 目录：
- `docs/RUST_TELEGRAM_BOT_GUIDE.md`
- `docs/rust-telegram-bot-plan.md`

## 部署

### Docker 部署

```bash
docker build -t telegram-bot .
docker run -v $(pwd)/logs:/app/logs telegram-bot
```

### systemd 部署

```ini
[Unit]
Description=Telegram Bot Service
After=network.target

[Service]
Type=simple
User=bot
WorkingDirectory=/opt/telegram-bot
ExecStart=/opt/telegram-bot/telegram-bot
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## 监控

### 查看日志

```bash
# 实时查看日志
tail -f logs/telegram-bot.log

# 查看数据库大小
ls -lh telegram_bot.db

# 查看表信息
sqlite3 telegram_bot.db \".schema messages"\"
```

## 常见问题

### Q: 数据库文件在哪里？

A: 默认在项目根目录的 `telegram_bot.db`，可通过 `DATABASE_URL` 环境变量修改。

### Q: 如何清空所有消息？

A: 直接删除数据库文件或使用 `./target/release/db-manager cleanup 0`

### Q: 如何迁移数据到其他数据库？

A: 使用 `./target/release/db-manager export` 导出为 JSON，然后导入到目标数据库。

### Q: 数据库会占用多少空间？

A: 每条消息约 500-1000 字节，10 万条消息约 50-100 MB。

### Q: 支持并发写入吗？

A: 是的，使用 SQLx 的连接池支持并发安全操作。

## 许可证

MIT License
