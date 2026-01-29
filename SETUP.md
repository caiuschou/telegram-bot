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

### 错误: Failed to get AI response / 令牌已过期或验证不正确 (code: 401)

**原因**: 调用对话模型 API（由 `OPENAI_API_KEY` + `OPENAI_BASE_URL` 指定）时，服务端返回 401，表示认证失败。常见情况：

- **API Key 过期或已撤销**：在对应平台重新生成 Key 并更新 `.env` 中的 `OPENAI_API_KEY`
- **Key 与 Base URL 不匹配**：例如 Key 是 A 平台的，`OPENAI_BASE_URL` 却指向 B 平台或代理，需保证 Key 和 URL 属于同一服务
- **Key 填写错误**：多/少空格、复制不完整、误用其他环境的 Key

**解决方法**:
1. 确认 `.env` 中 `OPENAI_API_KEY` 与 `OPENAI_BASE_URL` 对应同一服务（官方 OpenAI、代理或国内兼容接口等）
2. 在提供 Key 的平台检查该 Key 是否有效、未过期、未撤销
3. 若使用代理或自建接口，确认其要求的认证方式（如 Bearer token）与当前配置一致
4. 重新生成 Key 后，更新 `OPENAI_API_KEY` 并重启 Bot

### 错误: Failed to send message / Failed to send AI response（JSON: EOF, raw: ""）

**现象**: 日志出现 `Failed to send message` 或 `Failed to send AI response`，错误内容为 `An error while parsing JSON: EOF while parsing a value at line 1 column 0 (raw: "")`。

**原因**: 向 Telegram 发送消息时，请求的接口返回了**空响应体**，teloxide 按 JSON 解析失败。常见情况：

- **环境里设置了 `TELEGRAM_API_URL` 或 `TELOXIDE_API_URL`**：例如跑过集成测试后未取消，或误指向了 mock/代理，该地址返回了空内容而非 Telegram 标准 JSON。
- 网络或代理导致响应被截断、返回空 body。

**解决方法**:
1. **正式环境不要设置** Telegram 自定义 URL：在 `.env` 和当前 shell 中**不要**设置 `TELEGRAM_API_URL`、`TELOXIDE_API_URL`，让 Bot 使用默认 `api.telegram.org`。
2. 若刚跑过测试，在运行 Bot 的终端执行 `unset TELEGRAM_API_URL TELOXIDE_API_URL` 后重新启动。
3. 确认网络可访问 `api.telegram.org`，且无代理/防火墙把响应体清空。

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

通过 `run_bot`（如 `dbot`）启动时，会先创建 `logs` 目录并初始化日志：**日志会同时写入控制台和文件** `logs/telegram-bot.log`（路径相对运行时的当前工作目录）。若从项目根目录运行，完整路径为 `./logs/telegram-bot.log`。

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
