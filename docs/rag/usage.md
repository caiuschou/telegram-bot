# 使用示例

## 初始化记忆系统（内存+SQLite方案）

```rust
use memory::{MemoryStore, InMemoryVectorStore, EmbeddingService, OpenAIEmbedding};
use storage::MessageRepository;
use ai_integration::TelegramBotAI;
use bot_runtime::MemoryMiddleware;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化嵌入服务
    let embedding_service = Arc::new(OpenAIEmbedding::new(api_key));

    // 初始化SQLite存储（用于持久化）
    let sqlite_repo = MessageRepository::new("sqlite:./bot.db").await?;

    // 初始化内存向量存储
    let memory_store = Arc::new(InMemoryVectorStore::new(sqlite_repo));

    // 创建AI Bot
    let openai_client = OpenAIClient::new(api_key);
    let ai_bot = TelegramBotAI::new(openai_client)
        .with_memory(memory_store.clone(), embedding_service.clone());

    // 创建记忆中间件
    let middleware = MemoryMiddleware::new(
        memory_store,
        embedding_service,
        ContextBuilder::default()
            .with_max_tokens(2000)
            .with_recent(true)
            .with_relevant(true),
    );

    // 使用中间件处理消息
    // ...
    Ok(())
}
```

## 使用Lance方案（生产环境推荐）

```rust
use memory::{MemoryStore, LanceVectorStore, EmbeddingService, OpenAIEmbedding};

#[tokio::main]
async fn main() -> Result<()> {
    let embedding_service = Arc::new(OpenAIEmbedding::new(api_key));

    // Lance向量存储
    let memory_store = Arc::new(
        LanceVectorStore::new(
            "./data/memories.lance",
            embedding_service.clone(),
        ).await?
    );

    // 创建AI Bot
    let openai_client = OpenAIClient::new(api_key);
    let ai_bot = TelegramBotAI::new(openai_client)
        .with_memory(memory_store.clone(), embedding_service.clone());

    Ok(())
}
```

## 对话流程示例

```
用户: 我喜欢喝咖啡
Bot: [保存到用户偏好] 记住了，你喜欢喝咖啡。

用户: 推荐一杯饮品
Bot: [检索到偏好: 喜欢喝咖啡] 既然你喜欢咖啡，我推荐你尝试手冲咖啡...

用户: 最近怎么样？
Bot: [检索最近对话] 根据我们之前的对话，你刚提到了对咖啡的喜好...
```

## 用户偏好管理

```rust
// 标记为用户偏好
let entry = MemoryEntry::new(
    user_id,
    chat_id,
    MemoryRole::User,
    "我喜欢喝咖啡",
    embedding,
)
.with_preference(true); // 标记为偏好

memory_store.add_memory(&entry).await?;
```

## 重要信息标记

```rust
// 标记为重要信息
let entry = MemoryEntry::new(
    user_id,
    chat_id,
    MemoryRole::User,
    "我的邮箱是 example@email.com",
    embedding,
)
.with_important(true); // 标记为重要

memory_store.add_memory(&entry).await?;
```

## 自定义标签

```rust
// 添加自定义标签
let entry = MemoryEntry::new(
    user_id,
    chat_id,
    MemoryRole::User,
    "我在做人工智能项目",
    embedding,
)
.with_tags(vec!["工作".to_string(), "AI".to_string()]);

memory_store.add_memory(&entry).await?;
```

## 检索示例

```rust
// 检索相关历史
let relevant = memory_store.search_relevant(
    user_id,
    "推荐饮品",
    3  // top_k
).await?;

// 获取最近对话
let recent = memory_store.get_recent_context(user_id, 5).await?;

// 获取用户偏好
let preferences = memory_store.get_preferences(user_id).await?;

// 获取所有记忆
let all_memories = memory_store.get_user_memories(user_id).await?;
```

## 上下文构建示例

```rust
let context_builder = ContextBuilder::new(
    2000,   // max_context_tokens
    true,   // include_recent
    true,   // include_relevant
)
.with_recent_limit(5)
.with_relevant_top_k(3);

let context = context_builder
    .build_context(user_id, query, memory_store)
    .await?;
```

## 完整对话流程示例

```rust
// 用户发送消息
let user_message = "推荐一杯饮品";

// 中间件处理：保存消息 + 检索上下文
let context = middleware.process(user_id, chat_id, user_message).await?;

// AI生成回复
let response = ai_bot
    .get_ai_response_with_memory(user_message, user_id, context)
    .await?;

// AI回复自动保存到记忆库（在get_ai_response_with_memory中完成）
```
