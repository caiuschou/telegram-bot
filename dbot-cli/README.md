# Dbot CLI

Telegram Bot 运行工具，整合了 telegram-bot 的功能。

## 安装

```bash
# 从项目根目录编译
cargo build --release --package dbot-cli

# 二进制文件位于
./target/release/dbot
```

## 使用

### 查看帮助

```bash
./target/release/dbot --help
```

### 运行 Bot

```bash
# 使用环境变量中的 token
./target/release/dbot

# 指定 token
./target/release/dbot --token YOUR_BOT_TOKEN
```

## 环境变量

创建 `.env` 文件配置常用变量：

```env
# Telegram Bot Token
BOT_TOKEN=your_bot_token

# 数据库路径
DATABASE_URL=file:./telegram_bot.db
```

## 项目结构

```
dbot/
├── dbot-cli/          # CLI 工具（主入口）
│   ├── src/
│   │   └── main.rs    # Bot 运行逻辑
│   └── Cargo.toml
├── telegram-bot/      # Bot 库
│   ├── src/
│   │   └── lib.rs     # 库入口
│   └── Cargo.toml
└── storage/           # 数据持久化
    ├── src/
    │   └── ...
    └── Cargo.toml
```

## 工作流程

### 运行 Bot

1. 配置 `.env` 文件中的 `BOT_TOKEN`
2. 运行 `./target/release/dbot`
3. Bot 开始接收和保存消息

## 开发

```bash
# 开发模式
cargo run --package dbot-cli
```

## 故障排除

### Bot Token 错误

```
Error: BOT_TOKEN not set
```

解决方法：
1. 在 `.env` 中设置 `BOT_TOKEN`
2. 或使用 `--token` 参数

## 许可证

MIT License
