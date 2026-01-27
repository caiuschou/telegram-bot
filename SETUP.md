# Telegram Bot 快速设置指南

## 步骤 1: 获取 Bot Token

1. 在 Telegram 应用中打开 [@BotFather](https://t.me/BotFather)
2. 发送命令 `/newbot`
3. 按照提示输入 bot 名称和用户名
4. 保存获得的 bot token（格式：`123456789:ABCdefGHIjklMNOpqrsTUVwxyz`）

## 步骤 2: 配置环境变量

编辑项目根目录的 `.env` 文件：

```bash
nano .env
# 或使用其他编辑器
```

将 `BOT_TOKEN` 的值替换为你的真实 token：

```env
BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz
DATABASE_URL=file:./telegram_bot.db
RUST_LOG=info
```

## 步骤 3: 编译项目

```bash
# 从项目根目录运行
cargo build --release --bin telegram-bot
```

## 步骤 4: 运行 Bot

```bash
# 运行编译后的程序
./target/release/telegram-bot
```

## 验证 Bot 是否正常工作

1. 在 Telegram 中搜索你的 bot 用户名
2. 发送 `/start` 命令
3. 如果收到 "Welcome to Telegram Bot!" 的回复，说明配置成功

## 常见问题

### 错误: Api(InvalidToken)

**原因**: BOT_TOKEN 环境变量未设置或值不正确

**解决方法**:
1. 检查 `.env` 文件是否存在于项目根目录
2. 确认 BOT_TOKEN 的值是从 @BotFather 获取的真实 token
3. 重新运行程序

### 错误: Failed to connect to Telegram API

**原因**: 网络连接问题或防火墙阻止

**解决方法**:
1. 检查网络连接
2. 确认可以访问 `api.telegram.org`
3. 如果需要，配置代理

### 数据库错误

**原因**: 数据库文件权限问题或路径错误

**解决方法**:
1. 检查程序对目录有写入权限
2. 尝试删除现有的 `telegram_bot.db` 文件重新初始化

## 可用命令

| 命令 | 功能 |
|------|------|
| `/start` | 显示欢迎信息 |
| `/help` | 显示帮助信息 |
| `/echo <text>` | 回复消息 |
| `/stats` | 显示统计信息 |
| `/history` | 查看消息历史 |
| `/search <keyword>` | 搜索消息 |

## 数据库管理

项目包含 `db-manager` 工具用于管理数据库：

```bash
# 显示统计信息
./target/release/db-manager stats

# 查看用户历史
./target/release/db-manager history <user_id> [limit]

# 搜索消息
./target/release/db-manager search <keyword> [limit]

# 清理旧消息
./target/release/db-manager cleanup <days>

# 导出所有消息到 JSON
./target/release/db-manager export
```

## 日志查看

实时查看日志：

```bash
tail -f logs/telegram-bot.log
```

查看错误日志：

```bash
grep ERROR logs/telegram-bot.log
```

## 开发模式

直接运行开发版本：

```bash
cargo run --bin telegram-bot
```

## 更新 Bot Token

如果需要更新 bot token：

1. 编辑 `.env` 文件
2. 停止当前运行的 bot (Ctrl+C)
3. 重新运行 bot

## 安全建议

⚠️ **重要**: 永远不要将 `.env` 文件提交到代码仓库！

- `.env` 已在 `.gitignore` 中
- 仅在本地开发和生产服务器上使用真实的 token
- 定期更换 bot token 以提高安全性

## 进一步配置

### 修改数据库位置

编辑 `.env` 文件：

```env
DATABASE_URL=file:/path/to/custom/database.db
```

### 使用 PostgreSQL 或 MySQL

修改 `DATABASE_URL` 为 PostgreSQL 或 MySQL 连接字符串（需要修改 Cargo.toml 的依赖配置）。

### 修改日志级别

```env
# 最详细（用于调试）
RUST_LOG=trace

# 调试信息
RUST_LOG=debug

# 一般信息（默认）
RUST_LOG=info

# 仅警告和错误
RUST_LOG=warn

# 仅错误
RUST_LOG=error
```

## 支持

如有问题，请检查：

1. ✅ Bot token 是否正确
2. ✅ 网络连接是否正常
3. ✅ 磁盘写入权限是否正常
4. ✅ 日志文件中的错误信息

祝你使用愉快！🎉
