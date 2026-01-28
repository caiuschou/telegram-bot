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
├── ai-handlers/               # AI 处理器 ⭐
│   ├── src/
│   │   ├── ai_mention_detector.rs  # AI 提及检测器
│   │   ├── ai_response_handler.rs  # AI 响应处理器
│   │   └── lib.rs              # 库入口
│   └── Cargo.toml
├── openai-client/             # OpenAI 客户端
│   ├── src/
│   │   └── lib.rs             # ChatCompletion 和流式响应
│   └── Cargo.toml
├── telegram-bot/              # Telegram Bot 库 ⭐
│   ├── src/
│   │   ├── lib.rs             # 库入口
│   │   ├── config.rs          # 配置管理
│   │   ├── adapters.rs        # Telegram/Core 类型转换
│   │   ├── runner.rs          # Bot 运行时
│   │   └── telegram_impl.rs   # TelegramBot 实现 Bot trait
│   └── Cargo.toml
├── memory/                     # 内存管理 ⭐
│   ├── src/
│   │   ├── types.rs           # 内存类型定义
│   │   ├── store.rs           # MemoryStore trait
│   │   ├── context.rs         # 上下文构建
│   │   └── strategies.rs      # 上下文策略
│   └── Cargo.toml
├── crates/
│   ├── memory-inmemory/       # 内存存储实现
│   ├── memory-sqlite/         # SQLite 内存存储
│   └── memory-lance/          # Lance 向量存储
├── telegram-bot-ai/           # AI Bot 库
│   ├── src/
│   │   ├── lib.rs             # AI Bot 库
│   └── Cargo.toml
├── telegram-bot-examples/     # 示例项目
│   ├── src/
│   │   ├── echo.rs            # Echo 示例
│   │   └── clock.rs           # 时钟示例
│   ├── examples/
│   └── Cargo.toml
└── dbot-cli/                  # 统一 CLI 工具 ⭐
    ├── src/
    │   └── main.rs            # CLI 入口（薄层）
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

复制 `.env.example` 为 `.env` 并配置：

```env
# Telegram Bot Token
BOT_TOKEN=your_bot_token

# 数据库配置
DATABASE_URL=file:./telegram_bot.db

# OpenAI 配置
OPENAI_API_KEY=your_openai_api_key
OPENAI_BASE_URL=https://api.openai.com/v1
AI_MODEL=gpt-3.5-turbo

# AI 响应配置
AI_USE_STREAMING=false
AI_THINKING_MESSAGE=正在思考...

# 内存存储配置
MEMORY_STORE_TYPE=memory
MEMORY_SQLITE_PATH=./data/memory.db
```

**MEMORY_STORE_TYPE** 选项：
- `memory`: 内存存储（默认）
- `sqlite`: SQLite 持久化存储
- `lance`: Lance 向量存储（需启用 feature）

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
  - MemoryMiddleware - 内存管理
- ✅ 状态管理

### 内存管理
- ✅ 统一的 MemoryStore trait
- ✅ 多种存储实现
  - 内存存储
  - SQLite 持久化
  - Lance 向量存储
- ✅ 上下文构建
- ✅ 语义搜索
- ✅ 用户偏好管理

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
- ✅ 配置管理
- ✅ AI 查询处理

## 📖 使用示例

### 使用 CLI 工具

```bash
# 使用默认配置（从环境变量读取）
./target/release/dbot

# 使用命令行参数覆盖 token
./target/release/dbot --token your_bot_token
```

### 配置管理

```rust
use telegram_bot::BotConfig;

// 从环境变量加载配置
let config = BotConfig::load(None)?;

// 使用命令行参数覆盖 token
let config = BotConfig::load(Some("custom_token".to_string()))?;

// 配置字段包括：
// - bot_token: Bot token
// - database_url: 数据库 URL
// - log_file: 日志文件路径
// - openai_api_key: OpenAI API key
// - openai_base_url: OpenAI base URL
// - ai_model: AI 模型
// - ai_use_streaming: 是否使用流式响应
// - ai_thinking_message: 思考中提示消息
// - memory_store_type: 内存存储类型
// - memory_sqlite_path: SQLite 内存存储路径
```

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

### 使用 Telegram Bot AI

```rust
use telegram_bot_ai::TelegramBotAI;
use openai_client::OpenAIClient;

#[tokio::main]
async fn main() -> Result<()> {
    let client = OpenAIClient::new(api_key);
    let ai_bot = TelegramBotAI::new(bot_username.to_string(), client)
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
- [Memory 记忆管理](MEMORY.md) - 对话记忆、语义搜索、存储后端
- [dbot-cli 使用文档](dbot-cli/README.md)
- [telegram-bot 文档](telegram-bot/README.md) - 包含配置管理、类型转换、运行时等详细说明

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

**ai-handlers**: AI 处理器系统
- `AIDetectionHandler`: AI 提及检测器
- `AIQueryHandler`: AI 响应处理器
- `AIQuery`: AI 查询数据结构

**memory**: 内存管理系统
- `MemoryStore trait`: 统一存储接口
- 上下文构建：支持多种策略
- 语义搜索：基于向量存储
- 用户偏好管理

**crates**: 存储实现
- `memory-inmemory`: 内存存储实现
- `memory-sqlite`: SQLite 持久化存储
- `memory-lance`: Lance 向量存储（可选）

**telegram-bot**: Telegram Bot 完整实现
- `TelegramBot`: 实现 Bot trait
- `BotConfig`: 配置管理，环境变量封装
- `TelegramUserWrapper`/`TelegramMessageWrapper`: Telegram 到 Core 类型转换
- `run_bot()`: Bot 初始化和运行逻辑
- 消息持久化、AI 集成、内存存储

**dbot-cli**: 薄层 CLI 入口
- `Cli`: CLI 参数解析
- 调用 `telegram_bot::run_bot()`
- 不包含业务逻辑

### 设计原则

1. **模块化**: 每个模块职责单一，易于组合
2. **可扩展**: 基于 trait 的抽象，易于扩展
3. **类型安全**: 充分利用 Rust 类型系统
4. **异步优先**: 全异步设计，高并发支持
5. **薄层 CLI**: CLI 入口仅负责参数解析，业务逻辑在专门包中

### CLI 架构

`dbot-cli` 采用薄层设计：
- **职责**: CLI 参数解析和入口
- **委托**: 所有业务逻辑委托给 `telegram-bot` 包
- **配置**: 通过 `BotConfig` 统一管理环境变量
- **运行**: 调用 `telegram_bot::run_bot()` 启动 Bot

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
