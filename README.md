# Dbot - Telegram Bot 项目

一个功能完整、类型安全的 Telegram Bot 开发框架，基于 Rust 构建。

## 📚 项目结构

```
dbot/
├── Cargo.toml                   # Workspace 配置
├── README.md                    # 项目说明
├── SETUP.md                     # 快速设置指南
├── docs/                        # 文档目录
├── dbot-core/                  # 核心库 ⭐
│   ├── src/
│   │   ├── error.rs            # 错误类型定义
│   │   ├── types.rs            # 核心类型：User, Chat, Message
│   │   ├── bot.rs              # Bot trait 定义
│   │   └── logger.rs           # 日志模块
│   ├── Cargo.toml
│   └── README.md
├── storage/                    # 数据持久化 ⭐
│   ├── src/
│   │   ├── models.rs           # 数据模型：MessageRecord, MessageQuery
│   │   ├── repository.rs       # Repository trait
│   │   ├── message_repo.rs     # 消息仓库实现
│   │   └── sqlite_pool.rs      # SQLite 连接池
│   └── Cargo.toml
├── bot-runtime/               # Bot 运行时 ⭐
│   ├── src/
│   │   ├── handler.rs          # 消息处理器
│   │   ├── middleware.rs       # 中间件：LoggingMiddleware, AuthMiddleware
│   │   └── state.rs            # 状态管理
│   └── Cargo.toml
├── ai-integration/            # AI 集成
│   ├── src/
│   │   └── lib.rs             # TelegramBotAI 集成
│   └── Cargo.toml
├── openai-client/             # OpenAI 客户端
│   ├── src/
│   │   └── lib.rs             # ChatCompletion 和流式响应
│   └── Cargo.toml
├── telegram-bot/              # Telegram Bot 库
│   ├── src/
│   │   ├── lib.rs             # 库入口
│   │   └── telegram_impl.rs   # TelegramBot 实现 Bot trait
│   └── Cargo.toml
├── telegram-bot-ai/           # AI Bot 可执行程序
│   ├── src/
│   │   ├── main.rs            # AI Bot 入口
│   │   └── lib.rs             # AI Bot 库
│   └── Cargo.toml
├── telegram-bot-examples/     # 示例项目
│   ├── src/
│   │   ├── echo.rs            # Echo 示例
│   │   └── clock.rs           # 时钟示例
│   ├── examples/
│   └── Cargo.toml
└── dbot-cli/                  # 统一 CLI 工具 ⭐
    ├── src/
    │   └── main.rs            # CLI 入口
    ├── Cargo.toml
    └── README.md
```

## 🚀 快速开始

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 克隆项目

```bash
cd /Users/admin/dev/dbot
```

### 3. 编译项目

```bash
# 编译所有项目
cargo build --release

# 编译 CLI 工具
cargo build --release --package dbot-cli
```

### 4. 使用 CLI 工具

```bash
# 查看帮助
./target/release/dbot --help

# 运行 Bot
./target/release/dbot
```

### 5. 配置环境变量

复制 `dbot-cli/.env.example` 为 `.env` 并配置：

```env
# Telegram Bot Token
BOT_TOKEN=your_bot_token

# 数据库配置（可选）
DATABASE_URL=file:./telegram_bot.db
```

详细配置请查看 [SETUP.md](SETUP.md)

## 🎯 主要功能

### 核心架构
- ✅ 模块化设计，支持多种 Bot 实现
- ✅ 基于 trait 的抽象（Bot, Handler, Middleware）
- ✅ 类型安全的消息处理
- ✅ 异步运行时（tokio）

### 数据持久化
- ✅ SQLite 数据库支持
- ✅ 消息记录（MessageRecord）
- ✅ 灵活查询（MessageQuery）
- ✅ 统计分析（MessageStats）
- ✅ Repository 模式

### Bot 运行时
- ✅ 消息处理器
- ✅ 中间件系统
  - LoggingMiddleware - 日志记录
  - AuthMiddleware - 权限控制
- ✅ 状态管理

### AI 集成
- ✅ OpenAI ChatCompletion API
- ✅ 流式响应支持
- ✅ 自定义 base URL
- ✅ 多模型支持（gpt-3.5-turbo, gpt-4 等）
- ✅ TelegramBotAI 集成

### CLI 工具
- ✅ Bot 运行（整合了 telegram-bot 功能）
- ✅ 消息持久化
- ✅ 日志记录

## 📖 使用示例

### 创建自定义 Bot

```rust
use dbot_core::{Bot, Message, Result};
use bot_runtime::{MessageHandler, LoggingMiddleware};
use storage::MessageRepository;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化组件
    let repo = MessageRepository::new("sqlite:./bot.db").await?;
    let bot = telegram_bot::TelegramBot::new(token);
    
    // 创建处理器和中间件
    let handler = MessageHandler::new(repo.clone());
    let middleware = LoggingMiddleware;
    
    // 处理消息
    // ...
    
    Ok(())
}
```

### 使用 AI 集成

```rust
use ai_integration::TelegramBotAI;
use openai_client::OpenAIClient;

#[tokio::main]
async fn main() -> Result<()> {
    let client = OpenAIClient::new(api_key);
    let ai_bot = TelegramBotAI::new(client)
        .with_model("gpt-4".to_string());
    
    let response = ai_bot.get_ai_response("Hello!").await?;
    println!("AI Response: {}", response);
    
    Ok(())
}
```

### 使用中间件

```rust
use bot_runtime::{AuthMiddleware, LoggingMiddleware};

// 创建权限中间件
let auth = AuthMiddleware::new(vec![1234567890]);

// 创建日志中间件
let logger = LoggingMiddleware;

// 应用中间件
// ...

## 📖 详细文档

- [项目设置指南](SETUP.md)
- [dbot-cli 使用文档](dbot-cli/README.md)
- [telegram-bot 文档](telegram-bot/README.md)

## 🏗️ 架构设计

### 核心模块

**dbot-core**: 定义核心类型和接口
- `Bot trait`: 统一的 Bot 接口
- `Handler trait`: 消息处理器接口
- `Middleware trait`: 中间件接口
- `User`, `Chat`, `Message`: 核心数据结构

**storage**: 数据持久化层
- `Repository trait`: 通用仓库接口
- `MessageRepository`: 消息仓库实现
- 支持灵活查询和统计分析

**bot-runtime**: Bot 运行时系统
- `MessageHandler`: 消息持久化处理
- 中间件：日志、认证等

**ai-integration**: AI 功能集成
- `TelegramBotAI`: 集成 OpenAI 的 Bot
- 支持流式响应

### 设计原则

1. **模块化**: 每个模块职责单一，易于组合
2. **可扩展**: 基于 trait 的抽象，易于扩展
3. **类型安全**: 充分利用 Rust 类型系统
4. **异步优先**: 全异步设计，高并发支持

## 🔧 开发

```bash
# 运行 CLI 工具
cargo run --package dbot-cli

# 运行测试
cargo test

# 检查代码
cargo clippy

# 格式化代码
cargo fmt

# 构建所有项目
cargo build --release

# 构建特定项目
cargo build --release --package dbot-cli
```

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT License
