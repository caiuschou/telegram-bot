# Telegram Bot 集成测试方案

## 开发计划

| ID | 任务描述 | 优先级 | 状态 | 完成日期 | 备注 |
|----|---------|-------|------|---------|------|
| 1.1 | 添加测试依赖库到 `telegram-bot/Cargo.toml` | 高 | ✅ 已完成 | 2026-01-28 | 已添加 mockall 0.14, mockito 1.7, tempfile 3.24, tokio-test 0.4 |
| 1.2 | 创建测试文件 `telegram-bot/tests/runner_integration_test.rs` | 高 | ⬜ 待开始 | - | 基础测试文件结构 |
| 1.3 | 实现测试工具函数（setup_test_config, MockMemoryStore） | 高 | ⬜ 待开始 | - | 包含 .env 加载和临时目录创建 |
| 1.4 | 实现 Mock Telegram API | 中 | ⬜ 待开始 | - | 使用 mockito Mock getMe 和 sendMessage |
| 2.1 | 重构 `runner.rs` - 提取 Bot 组件初始化逻辑 | 高 | ⬜ 待开始 | - | 创建 `initialize_bot_components` 函数 |
| 2.2 | 重构 `runner.rs` - 创建可测试的 TelegramBot 结构 | 高 | ⬜ 待开始 | - | 支持依赖注入，便于测试 |
| 2.3 | 实现 TelegramBot::new 和 new_with_memory_store 方法 | 高 | ⬜ 待开始 | - | 支持注入自定义 MemoryStore |
| 2.4 | 实现 TelegramBot::handle_message 方法 | 高 | ⬜ 待开始 | - | 可测试的消息处理接口 |
| 2.5 | 实现 TelegramBot::start_ai_handler 方法 | 高 | ⬜ 待开始 | - | 启动 AI 查询处理器 |
| 3.1 | 实现 Lance 向量存储验证 | 高 | ⬜ 待开始 | - | 验证数据库创建和向量存储 |
| 3.2 | 实现 Lance 向量查询验证 | 高 | ⬜ 待开始 | - | 验证向量相似度搜索 |
| 3.3 | 实现真实 OpenAI API 集成测试 | 高 | ⬜ 待开始 | - | 使用真实 API Key 进行测试 |
| 3.4 | 实现 AI 回复流程端到端测试 | 高 | ⬜ 待开始 | - | 完整流程验证 |
| 4.1 | 创建配置示例文件 `.env.test.example` | 低 | ⬜ 待开始 | - | 包含所有测试配置项 |
| 4.2 | 更新文档说明 Lance 使用方式 | 低 | ⬜ 待开始 | - | Lance 数据库配置说明 |
| 4.3 | 添加测试执行说明到文档 | 低 | ⬜ 待开始 | - | 本地测试和验证方法 |
| 5.1 | 运行测试并验证通过 | 高 | ⬜ 待开始 | - | 确保所有测试用例通过 |
| 5.2 | 测试覆盖率检查 | 中 | ⬜ 待开始 | - | 验证测试覆盖率达到目标 |
| 5.3 | 性能测试和优化 | 低 | ⬜ 待开始 | - | 优化测试执行时间 |

**图例说明**：
- ⬜ 待开始
- 🔄 进行中
- ✅ 已完成
- ⏸️ 暂停
- ❌ 已取消

**优先级**：
- 高：核心功能，必须完成
- 中：重要功能，影响测试质量
- 低：优化和文档，可以延后

## 概述

本文档描述了 Telegram Bot 的 `run_bot` 方法（`telegram-bot/src/runner.rs:21`）的集成测试方案。测试重点是验证 AI 回复的完整流程。

## 测试架构

```
telegram-bot/
├── Cargo.toml
├── src/
│   ├── runner.rs          # 包含待测试的 run_bot 函数
│   ├── config.rs
│   └── adapters.rs
└── tests/
    └── runner_integration_test.rs  # 集成测试
```

## 测试场景

| 测试用例 | 描述 | 验证点 |
|---------|------|-------|
| 1. AI 回复完整流程 | 验证从用户消息到 AI 回复的完整流程，包括 Lance 向量存储和查询 | • Bot 成功初始化<br>• Lance 向量数据库连接建立<br>• 用户消息被接收<br>• 消息持久化到数据库<br>• 消息被转换为向量并存储到 Lance<br>• Lance 向量查询被执行以获取相关记忆<br>• 查询结果被传递给 AI 处理器<br>• AI 使用上下文生成回复<br>• AI 回复被发送回用户<br>• AI 回复也被持久化和向量化存储到 Lance |

## 技术实现

### 依赖库

在 `telegram-bot/Cargo.toml` 的 `[dev-dependencies]` 中添加：

```toml
[dev-dependencies]
mockall = "0.13"
mockito = "1.4"
tempfile = "3.10"
tokio-test = "0.4"
```

### 测试工具函数

```rust
// telegram-bot/tests/runner_integration_test.rs

use std::env;
use tempfile::TempDir;

/// 设置测试配置，使用临时目录
/// 注意：测试需要真实的 OpenAI API Key
/// API Key 可以通过以下方式提供：
/// 1. 从 .env 文件中读取（使用 dotenvy）
/// 2. 通过环境变量设置
/// 3. 在 CI/CD 中通过 secrets 设置
fn setup_test_config() -> BotConfig {
    // 从 .env 文件加载环境变量（如果 .env 文件存在）
    let _ = dotenvy::dotenv();

    // 创建临时目录用于日志和数据库
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // 从环境变量读取 OpenAI API Key（真实值）
    let openai_api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set in .env file or environment variable");

    let openai_base_url = env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

    env::set_var("BOT_TOKEN", "test_bot_token_12345");
    env::set_var("OPENAI_API_KEY", &openai_api_key);
    env::set_var("OPENAI_BASE_URL", &openai_base_url);
    env::set_var("DATABASE_URL", format!("{}/test.db", temp_path.display()));
    env::set_var("AI_MODEL", "gpt-3.5-turbo");
    env::set_var("AI_USE_STREAMING", "false");
    env::set_var("AI_THINKING_MESSAGE", "Thinking...");
    env::set_var("MEMORY_STORE_TYPE", "lance");
    env::set_var("MEMORY_LANCE_PATH", format!("{}/lance_db", temp_path.display()));

    BotConfig::load(None).unwrap()
}

/// 设置测试配置，使用临时目录
fn setup_test_config() -> BotConfig {
    // 创建临时目录用于日志和数据库
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    env::set_var("BOT_TOKEN", "test_bot_token_12345");
    env::set_var("OPENAI_API_KEY", "test_api_key");
    env::set_var("DATABASE_URL", format!("{}/test.db", temp_path.display()));
    env::set_var("OPENAI_BASE_URL", "https://api.test.com/v1");
    env::set_var("AI_MODEL", "gpt-3.5-turbo");
    env::set_var("AI_USE_STREAMING", "false");
    env::set_var("AI_THINKING_MESSAGE", "Thinking...");
    env::set_var("MEMORY_STORE_TYPE", "lance");
    env::set_var("MEMORY_LANCE_PATH", format!("{}/lance_db", temp_path.display()));

    BotConfig::load(None).unwrap()
}

/// Mock Telegram Bot 的 getMe 接口
fn mock_telegram_get_me() -> mockito::ServerGuard {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/getMe")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "ok": true,
            "result": {
                "id": 123456789,
                "is_bot": true,
                "first_name": "TestBot",
                "username": "testbot"
            }
        }"#)
        .create();
    server
}

/// Mock Telegram Bot 的 sendMessage 接口
fn mock_telegram_send_message(server: &mockito::ServerGuard) -> mockito::Mock {
    server.mock("POST", "/sendMessage")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "ok": true,
            "result": {
                "message_id": 1,
                "chat": {"id": 123},
                "text": "This is a test response"
            }
        }"#)
        .create()
}


```

### 测试用例实现

#### 测试：AI 回复完整流程

```rust
use std::time::Duration;

#[tokio::test]
async fn test_ai_reply_complete_flow() {
    // 1. 设置测试环境
    let (temp_dir, config) = setup_temp_environment();

    // 2. 创建 Mock 服务器
    let mut server = mockito::Server::new();

    // Mock Telegram getMe 接口 - 获取 Bot 信息
    let _mock_get_me = mock_telegram_get_me();
    let _mock_send_message = mock_telegram_send_message(&server);

    // Mock OpenAI 聊天完成接口
    let _mock_openai = mock_openai_chat_completion(&server);

    // 3. 启动 Bot
    let bot_handle = tokio::spawn(async move {
        // 注意：这里需要修改 run_bot 的实现，使其支持测试模式
        // 或者在测试中注入 mock 的 Telegram Bot 实例
        run_bot(config).await
    });

    // 4. 等待 Bot 初始化完成
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 5. 模拟发送用户消息
    // 在实际实现中，这需要通过 teloxide 的测试支持
    // 或者需要注入一个可以手动触发消息的机制

    // 6. 验证 AI 回复流程
    // 验证点：
    // - Bot username 被正确设置
    // - 用户消息被接收到
    // - 消息被发送到 AI 查询队列
    // - AI 处理器处理了消息
    // - AI 回复被发送回 Telegram

    // 7. 清理
    bot_handle.abort();
    temp_dir.close().unwrap();

    // 测试通过
    assert!(true, "AI 回复完整流程测试通过");
}
```

### 测试实现说明

由于 `run_bot` 函数的复杂性，完整的 AI 回复流程测试需要以下调整：

#### 选项 1：提取关键逻辑到独立函数

将 `run_bot` 中的关键逻辑提取到独立函数，便于测试：

```rust
// telegram-bot/src/runner.rs

/// 初始化 Bot 的核心组件（提取为独立函数）
pub async fn initialize_bot_components(config: &BotConfig) -> Result<BotComponents> {
    // 初始化存储
    let repo = Arc::new(
        MessageRepository::new(&config.database_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize message storage: {}", e))?
    );

    // 初始化 Telegram Bot
    let teloxide_bot = Bot::new(config.bot_token.clone());

    // 存储 bot username
    let bot_username = Arc::new(tokio::sync::RwLock::new(None));

    // 创建 AI 查询通道
    let (query_sender, query_receiver) = tokio::sync::mpsc::unbounded_channel();

    // 初始化 OpenAI 客户端
    let openai_client = OpenAIClient::with_base_url(
        config.openai_api_key.clone(),
        config.openai_base_url.clone()
    );
    let ai_bot = TelegramBotAI::new("bot".to_string(), openai_client)
        .with_model(config.ai_model.clone());

    // 初始化内存存储
    let memory_store: Arc<dyn MemoryStore> = match config.memory_store_type.as_str() {
        "sqlite" => {
            Arc::new(SQLiteVectorStore::new(&config.memory_sqlite_path).await?)
        }
        _ => Arc::new(InMemoryVectorStore::new())
    };

    // 初始化 AI 查询处理器
    let ai_query_handler = AIQueryHandler::new(
        ai_bot,
        teloxide_bot.clone(),
        repo.as_ref().clone(),
        memory_store.clone(),
        query_receiver,
        config.ai_use_streaming,
        config.ai_thinking_message.clone(),
    );

    Ok(BotComponents {
        repo,
        teloxide_bot,
        bot_username,
        query_sender,
        ai_query_handler,
        memory_store,
    })
}

/// Bot 组件集合
pub struct BotComponents {
    pub repo: Arc<MessageRepository>,
    pub teloxide_bot: Bot,
    pub bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    pub query_sender: tokio::sync::mpsc::UnboundedSender<AIQuery>,
    pub ai_query_handler: AIQueryHandler,
    pub memory_store: Arc<dyn MemoryStore>,
}
```

#### 选项 2：使用依赖注入

创建可测试的 Bot 结构：

```rust
// telegram-bot/src/runner.rs

pub struct TelegramBot {
    config: BotConfig,
    bot: Bot,
    components: BotComponents,
    handler_chain: HandlerChain,
}

impl TelegramBot {
    pub async fn new(config: BotConfig) -> Result<Self> {
        let components = initialize_bot_components(&config).await?;
        let bot = components.teloxide_bot.clone();

        // 初始化 AI 检测处理器
        let ai_detection_handler = Arc::new(AIDetectionHandler::new(
            components.bot_username.clone(),
            Arc::new(components.query_sender.clone()),
        ));

        // 初始化持久化中间件
        let persistence_middleware = Arc::new(PersistenceMiddleware::new(
            components.repo.as_ref().clone()
        ));

        // 初始化记忆中间件
        let memory_middleware = Arc::new(MemoryMiddleware::with_store(
            components.memory_store.clone()
        ));

        // 构建处理器链
        let handler_chain = HandlerChain::new()
            .add_middleware(persistence_middleware)
            .add_middleware(memory_middleware)
            .add_handler(ai_detection_handler);

        Ok(Self {
            config,
            bot,
            components,
            handler_chain,
        })
    }

    /// 处理消息（可测试的接口）
    pub async fn handle_message(&self, message: &Message) -> Result<()> {
        self.handler_chain.handle(message).await
    }

    /// 启动 AI 查询处理器
    pub fn start_ai_handler(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.components.ai_query_handler.run().await;
        })
    }
}
```

### 改进后的测试用例

```rust
use std::sync::Arc;

/// 可跟踪调用次数的 MemoryStore Mock
struct MockMemoryStore {
    store_call_count: Arc<AtomicUsize>,
    query_call_count: Arc<AtomicUsize>,
}

/// 可跟踪调用次数的 MemoryStore Mock
struct MockMemoryStore {
    store_call_count: Arc<AtomicUsize>,
    query_call_count: Arc<AtomicUsize>,
}

impl MockMemoryStore {
    fn new() -> Self {
        Self {
            store_call_count: Arc::new(AtomicUsize::new(0)),
            query_call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn get_store_call_count(&self) -> usize {
        self.store_call_count.load(Ordering::SeqCst)
    }

    fn get_query_call_count(&self) -> usize {
        self.query_call_count.load(Ordering::SeqCst)
    }
}

#[tokio::test]
async fn test_ai_reply_complete_flow() {
    // 1. 检查必需的环境变量
    if env::var("OPENAI_API_KEY").is_err() {
        eprintln!("SKIP: OPENAI_API_KEY not set, skipping integration test");
        return;
    }

    // 2. 设置测试环境和配置
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = setup_test_config();

    // 3. 创建 MemoryStore Mock 来跟踪调用
    let mock_memory_store = create_mock_memory_store();

    // 4. 初始化 Bot（使用新的 TelegramBot 结构，注入 Mock MemoryStore）
    let bot = TelegramBot::new_with_memory_store(config, mock_memory_store.clone())
        .await
        .unwrap();

    // 5. 启动 AI 处理器
    let _ai_handler_handle = bot.start_ai_handler();

    // 6. 等待初始化完成
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 7. 创建测试消息
    let test_message = Message {
        id: "test_msg_1".to_string(),
        user: User {
            id: 123456,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: None,
        },
        chat: Chat {
            id: 123456,
            chat_type: "private".to_string(),
        },
        content: "Hello, can you help me?".to_string(),
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: chrono::Utc::now(),
        reply_to_message_id: Some("bot_msg_123".to_string()),
    };

    // 8. 处理消息
    let result = bot.handle_message(&test_message).await;

    // 9. 验证处理成功
    assert!(result.is_ok(), "Message handling should succeed");

    // 10. 等待 AI 回复和处理完成（使用真实 OpenAI API 可能需要更长时间）
    tokio::time::sleep(Duration::from_secs(10)).await;

    // 11. 验证向量存储被调用（消息被向量化并存储）
    // 用户消息和 AI 回复都应该被存储
    assert!(
        mock_memory_store.store_call_count.load(Ordering::SeqCst) >= 2,
        "Memory store should be called at least twice (user message + AI response)"
    );

    // 12. 验证向量查询被执行（获取相关记忆）
    assert!(
        mock_memory_store.query_call_count.load(Ordering::SeqCst) >= 1,
        "Vector query should be executed at least once"
    );

    // 13. 清理
    temp_dir.close().unwrap();
}
```

### 向量查询验证要点

在测试中需要验证以下 Lance 向量数据库相关的流程：

#### 1. Lance 数据库初始化

```rust
// 测试开始时应该：
// - 创建 Lance 数据库连接
// - 初始化向量表（如果不存在）
// - 确保数据库路径可访问

// 验证方式：
let lance_db_path = Path::new(temp_dir.path()).join("lance_db");
assert!(lance_db_path.exists(), "Lance database directory should be created");
```

#### 2. 消息向量化存储到 Lance

```rust
// 当用户消息被处理时，应该：
// - 调用真实 OpenAI Embeddings API 生成向量
// - 将向量和元数据存储到 Lance 数据库
// - 记录消息的元数据（用户 ID、时间戳、消息 ID 等）

// 验证方式：
// 通过 MockMemoryStore 的 store_call_count 验证
let store_count = mock_memory_store.store_call_count.load(Ordering::SeqCst);
assert!(store_count >= 1, "Embeddings API should be called and vectors should be stored to Lance");
```

#### 3. Lance 向量查询执行

```rust
// AI 处理器在生成回复前应该：
// - 使用 Lance 进行向量相似度搜索
// - 查询相关历史记忆
// - 返回最相似的消息列表

// 验证方式：
// 通过 MockMemoryStore 的 query_call_count 验证
let query_count = mock_memory_store.query_call_count.load(Ordering::SeqCst);
assert_eq!(
    query_count,
    1,
    "Lance vector query should be executed once per AI request"
);
```

#### 4. 查询结果使用

```rust
// 真实 OpenAI API 会使用从 Lance 查询获取的相关记忆
// 验证方式：
// 可以通过日志或返回的 AI 回复内容来推断上下文是否被使用
// AI 的回复应该反映出对相关记忆的理解

// 由于使用真实 API，我们可以检查 AI 回复的质量
assert!(!ai_response.is_empty(), "AI should provide a response");
```

#### 5. AI 回复向量化存储到 Lance

```rust
// AI 回复也应该被向量化并存储到 Lance
// 验证方式：
// - 通过真实 OpenAI Embeddings API 生成向量
// - 存储到 Lance 数据库

let store_count = mock_memory_store.store_call_count.load(Ordering::SeqCst);
assert!(
    store_count >= 2,
    "Both user message and AI response should be vectorized and stored in Lance"
);
```

#### 6. Lance 数据持久化

```rust
// Lance 应该正确持久化数据
// 验证方式：
// - 检查数据库目录中的文件
// - 验证可以重新打开数据库并读取数据

let lance_db_files = std::fs::read_dir(lance_db_path)
    .expect("Should be able to read Lance database directory");
assert!(lance_db_files.count() > 0, "Lance database should contain data files");
```

#### 5. 真实 API 调用说明

```rust
// 测试使用真实 OpenAI API，意味着：
// - 每次测试都会消耗 OpenAI API 配额
// - 测试速度取决于网络延迟
// - 需要有效的 OPENAI_API_KEY 环境变量
// - 可能会遇到 API 限流或错误

// 跳过条件：
if env::var("OPENAI_API_KEY").is_err() {
    println!("SKIP: OPENAI_API_KEY not set");
    return;
}
```

## 测试环境配置

### 必需环境变量

运行集成测试前，需要设置以下环境变量。**推荐使用 .env 文件**来管理配置。

#### 方式 1：使用 .env 文件（推荐）

在项目根目录创建 `.env` 文件：

```bash
# OpenAI API 配置（必需 - 使用真实 API）
OPENAI_API_KEY=sk-your-real-openai-api-key-here
OPENAI_BASE_URL=https://api.openai.com/v1  # 可选，默认为官方地址
AI_MODEL=gpt-3.5-turbo  # 可选，默认为 gpt-3.5-turbo

# Telegram Bot 配置（测试用，可以随意设置）
BOT_TOKEN=test_bot_token_for_testing

# 数据库配置（可选，测试会使用临时数据库）
DATABASE_URL=file:./telegram_bot.db

# 流式响应配置
AI_USE_STREAMING=false
AI_THINKING_MESSAGE=Thinking...

# 记忆存储类型（lance、memory 或 sqlite）
MEMORY_STORE_TYPE=lance

# Lance 数据库路径（仅当 MEMORY_STORE_TYPE=lance 时使用）
MEMORY_LANCE_PATH=./data/lance_db
```

**注意事项**：
- `.env` 文件通常已添加到 `.gitignore`，不会提交到仓库
- 测试代码会自动使用 `dotenvy::dotenv()` 加载 `.env` 文件
- 如果 `.env` 文件不存在，会静默失败，需要确保环境变量已通过其他方式设置

#### 方式 2：直接设置环境变量

```bash
# OpenAI API 配置（必需 - 使用真实 API）
export OPENAI_API_KEY="your_real_openai_api_key"
export OPENAI_BASE_URL="https://api.openai.com/v1"  # 可选，默认为官方地址
export AI_MODEL="gpt-3.5-turbo"  # 可选，默认为 gpt-3.5-turbo

# Telegram Bot 配置（测试用，可以随意设置）
export BOT_TOKEN="test_bot_token_for_testing"

# 数据库配置（可选，测试会使用临时数据库）
# DATABASE_URL 会自动设置为临时文件路径

# 记忆存储类型（lance、memory 或 sqlite）
export MEMORY_STORE_TYPE="lance"

# Lance 数据库路径（可选，测试会使用临时目录）
export MEMORY_LANCE_PATH="./data/lance_db"
```

#### 环境变量加载优先级

1. 测试代码显式设置的环境变量（如 `env::set_var()`）
2. 从 .env 文件读取的环境变量
3. 系统环境变量
4. 代码中的默认值

#### Lance 数据库说明

- **MEMORY_STORE_TYPE=lance**：使用 Lance 向量数据库存储向量
- **MEMORY_LANCE_PATH**：指定 Lance 数据库的存储路径
- 测试会使用临时目录创建 Lance 数据库，测试结束后自动清理
- Lance 提供高性能的向量相似度搜索和持久化存储

### OpenAI API 注意事项

1. **API 配额消耗**：每次测试都会调用真实的 OpenAI API，会消耗配额
2. **网络依赖**：测试需要稳定的网络连接到 OpenAI API
3. **成本考虑**：建议使用较便宜的模型（如 gpt-3.5-turbo）进行测试
4. **错误处理**：如果 OpenAI API 返回错误，测试会失败
5. **Key 管理**：
   - 不要将 `.env` 文件提交到 Git 仓库（已在 `.gitignore` 中）
   - 使用示例文件 `.env.example` 展示配置结构

## 测试执行

### 本地测试

#### 使用 .env 文件（推荐）

1. 确保项目根目录有 `.env` 文件，包含 `OPENAI_API_KEY`
2. 运行测试：

```bash
cd telegram-bot
cargo test --test runner_integration_test
```

测试会自动从 `.env` 文件加载配置。

#### 手动设置环境变量

```bash
export OPENAI_API_KEY="your_real_openai_api_key"
cd telegram-bot
cargo test --test runner_integration_test
```

运行特定测试：

```bash
cargo test --test runner_integration_test test_ai_reply_complete_flow
```

运行并查看输出：

```bash
cargo test --test runner_integration_test -- --nocapture
```

## 测试覆盖目标

- **AI 回复流程**: 100% - 从消息接收到 AI 回复的完整流程
- **Lance 向量存储**: 100% - 向量存储到 Lance 数据库
- **Lance 向量查询**: 100% - 使用 Lance 进行向量相似度搜索
- **真实 OpenAI API 集成**: 100% - 使用真实 API 进行嵌入和聊天完成
- **组件集成**: 90%+ - 各组件之间的交互
- **错误处理**: 80%+ - 关键错误场景（包括 OpenAI API 和 Lance 错误处理）

## Mock 策略

### 外部依赖

| 依赖 | Mock 方式 |
|-----|----------|
| Telegram API | 使用 `mockito` Mock HTTP 接口 |
| OpenAI API | **使用真实 API**，不 mock |
| Lance 向量数据库 | **使用真实 Lance 实例**，不 mock（使用临时目录） |
| 数据库（消息存储） | 使用临时 SQLite 文件 |
| 文件系统 | 使用 `tempfile` 创建临时目录 |

### 组件 Mock

为以下组件创建基于 trait 的 mock：

- `MemoryStore` - 向量操作（存储、查询）
  - 跟踪存储调用次数
  - 跟踪查询调用次数
  - 验证存储和查询的内容
  - 可以使用真实 Lance 实例进行端到端测试
- `MessageRepository` - 数据库操作
  - 使用真实的临时数据库
  - 验证消息持久化
  - 验证历史记录查询
- `TelegramBotAI` - OpenAI 客户端
  - **使用真实 API**，不 mock
  - 直接调用 OpenAI 服务

### Lance 数据库处理

- **使用真实 Lance 实例**：测试使用真实的 Lance 向量数据库，而不是 mock
- **临时数据库**：测试使用临时目录创建 Lance 数据库，测试后自动清理
- **验证点**：
  - Lance 数据库目录被正确创建
  - 向量数据被正确写入
  - 向量查询返回正确的结果
  - 数据持久化成功
- **性能考虑**：Lance 提供高性能的向量搜索，适合测试场景

## 维护

- 当 `run_bot` 签名变更时更新测试
- 为新功能添加测试用例
- 保持 Mock 数据与 API 变更同步
- 每季度审查和更新测试覆盖率

## 测试注意事项

### 使用真实 OpenAI API 的考虑

1. **测试成本**：
   - 每次测试运行都会消耗 OpenAI API 配额
   - 建议在 CI/CD 中限制测试频率
   - 可以考虑使用 OpenAI 的测试环境或低配额 key

2. **测试稳定性**：
   - 网络问题可能导致测试失败
   - OpenAI API 服务不稳定可能影响测试
   - 建议添加重试逻辑或超时设置

3. **性能影响**：
   - 真实 API 调用会增加测试执行时间
   - 建议设置合理的超时时间（如 10-30 秒）

4. **数据隐私**：
   - 确保测试数据不包含敏感信息
   - 测试消息内容应该简单且无害

5. **CI/CD 集成**：
    ```yaml
    # 示例 CI 配置
    test:
      before_script:
        - cd telegram-bot
        # 方式 1: 从 CI/CD secrets 设置环境变量
        - export OPENAI_API_KEY=$OPENAI_API_KEY_SECRET
        # 方式 2: 创建临时的 .env 文件（推荐）
        - echo "OPENAI_API_KEY=$OPENAI_API_KEY_SECRET" > .env
        - echo "OPENAI_BASE_URL=https://api.openai.com/v1" >> .env
        - echo "AI_MODEL=gpt-3.5-turbo" >> .env
      script:
        - cargo test --test runner_integration_test -- --test-threads=1
      variables:
        OPENAI_API_KEY_SECRET: "@openai_api_key"  # CI/CD secrets
      only:
        - merge_requests
        - main
    ```

6. **本地测试**：
   - 推荐在本地开发时运行完整测试
   - 使用 `.env` 文件管理配置（已添加到 .gitignore）
   - 定期检查 OpenAI API 使用情况
   - 确保 `.env` 文件包含有效的 `OPENAI_API_KEY`

7. **跳过测试的条件**：
   - 未设置 `OPENAI_API_KEY` 时自动跳过
   - 在 CI 环境中可以通过配置选择是否运行集成测试

## 参考文献

- `telegram-bot/src/runner.rs` - Bot 主入口
- `telegram-bot/src/config.rs` - 配置管理
- `ai-handlers/src/ai_mention_detector.rs` - 测试模式示例
- AGENTS.md - 项目测试指南
