# Telegram Bot 快速设置

## 1. 获取 Bot Token

在 Telegram 中打开 [@BotFather](https://t.me/BotFather)，发送 `/newbot`，按提示创建并保存 token。

## 2. 配置环境变量

复制 `.env.example` 为 `.env`，至少设置：

```env
BOT_TOKEN=你的token
DATABASE_URL=file:./telegram_bot.db
RUST_LOG=info
```

AI 与记忆相关见根目录 `.env.example`（OPENAI_API_KEY、MEMORY_STORE_TYPE、LANCE_DB_PATH 等）。

## 3. 编译与运行

```bash
cargo build --release --package dbot-cli
./target/release/dbot
```

或运行 telegram-bot 二进制：`cargo build --release --bin telegram-bot`，`./target/release/telegram-bot`。

## 4. 验证

在 Telegram 中搜索 bot 用户名，发送 `/start`；若收到欢迎回复则配置成功。

## 常见问题

- **Api(InvalidToken)**：检查 `.env` 中 BOT_TOKEN 是否正确、是否来自 @BotFather。
- **Failed to connect to Telegram API**：检查网络、能否访问 api.telegram.org；必要时配置代理。
- **数据库错误**：检查 DATABASE_URL 路径与目录写权限。
- **401 / 令牌已过期**：OPENAI_API_KEY 或智谱 Key 过期/错误；确认 OPENAI_BASE_URL 与 Key 对应同一服务。
- **JSON EOF / 空响应**：勿在正式环境设置 TELEGRAM_API_URL、TELOXIDE_API_URL（测试 mock 会返回空）；用默认 api.telegram.org。
