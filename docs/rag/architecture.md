# 架构设计

## 模块结构

```
memory/                       # 新增：记忆管理模块
├── src/
│   ├── lib.rs               # 记忆trait和核心实现
│   ├── conversation.rs      # 对话记忆（ConversationMemory）
│   ├── embedding.rs         # 嵌入服务（OpenAI）
│   ├── retrieval.rs         # 检索器（语义搜索）
│   ├── context.rs           # 上下文构建器
│   └── types.rs             # 核心类型定义
└── Cargo.toml

telegram-bot-ai/             # 扩展：AI集成模块
├── src/
│   ├── lib.rs               # TelegramBotAI
│   └── Cargo.toml

bot-runtime/                 # 扩展：运行时
├── src/
│   ├── memory_middleware.rs # 记忆中间件（新增）
│   └── ...
└── Cargo.toml
```

## 核心组件

### 1. memory 模块

**功能职责**：
- 对话消息的存储和向量化
- 用户偏好和重要信息的记忆
- 语义检索相关历史
- 上下文窗口管理

**核心数据结构**：

```rust
// 记忆条目
pub struct MemoryEntry {
    pub id: String,
    pub user_id: i64,
    pub chat_id: i64,
    pub role: MemoryRole,        // User | Assistant | System
    pub content: String,
    pub embedding: Vec<f32>,
    pub timestamp: i64,
    pub metadata: MemoryMetadata,
}

pub struct MemoryMetadata {
    pub is_preference: bool,     // 是否为用户偏好
    pub is_important: bool,      // 是否为重要信息
    pub conversation_id: String,
    pub tags: Vec<String>,       // 自定义标签
}

// 记忆trait
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn add_memory(&self, entry: &MemoryEntry) -> Result<()>;
    async fn add_memories(&self, entries: &[MemoryEntry]) -> Result<()>;
    async fn search_relevant(
        &self,
        user_id: i64,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<MemoryEntry>>;

    async fn get_recent_context(
        &self,
        user_id: i64,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>>;

    async fn get_preferences(&self, user_id: i64) -> Result<Vec<MemoryEntry>>;
    async fn get_user_memories(&self, user_id: i64) -> Result<Vec<MemoryEntry>>;
}

// 上下文构建器
pub struct ContextBuilder {
    max_context_tokens: usize,
    include_recent: bool,
    include_relevant: bool,
}

impl ContextBuilder {
    pub async fn build_context(
        &self,
        user_id: i64,
        query: &str,
        memory_store: Arc<dyn MemoryStore>,
    ) -> Result<String>;
}
```

### 2. 记忆中间件

**功能职责**：
- 自动保存对话到记忆库
- 在处理消息前检索相关上下文
- 注入上下文到AI提示词

**核心实现**：

```rust
pub struct MemoryMiddleware {
    memory_store: Arc<dyn MemoryStore>,
    embedding_service: Arc<dyn EmbeddingService>,
    context_builder: ContextBuilder,
}

#[async_trait]
impl Middleware for MemoryMiddleware {
    async fn process(
        &self,
        user_id: i64,
        chat_id: i64,
        content: &str,
    ) -> Result<Option<String>> {
        // 1. 保存用户消息到记忆
        let user_entry = MemoryEntry::new(
            user_id,
            chat_id,
            MemoryRole::User,
            content,
            self.embedding_service.embed(content).await?,
        );
        self.memory_store.add_memory(&user_entry).await?;

        // 2. 检索相关上下文
        let context = self.context_builder
            .build_context(user_id, content, self.memory_store.clone())
            .await?;

        Ok(Some(context))
    }
}
```

### 3. AI Bot 增强

**功能职责**：
- 接收记忆中间件提供的上下文
- 构建完整的AI提示词
- 生成回复并保存到记忆

**核心接口**：

```rust
pub struct TelegramBotAI {
    openai_client: OpenAIClient,
    model: String,
    memory_store: Option<Arc<dyn MemoryStore>>,
    embedding_service: Option<Arc<dyn EmbeddingService>>,
}

impl TelegramBotAI {
    pub async fn get_ai_response_with_memory(
        &self,
        question: &str,
        user_id: i64,
        context: Option<String>,
    ) -> Result<String> {
        let messages = self.build_messages(question, context).await?;
        let response = self.openai_client.chat_completion(&self.model, messages).await?;

        // 保存AI回复到记忆
        if let Some(store) = &self.memory_store {
            if let Some(embedding) = &self.embedding_service {
                let assistant_entry = MemoryEntry::new(
                    user_id,
                    user_id, // 简化处理
                    MemoryRole::Assistant,
                    &response,
                    embedding.embed(&response).await?,
                );
                store.add_memory(&assistant_entry).await?;
            }
        }

        Ok(response)
    }

    async fn build_messages(
        &self,
        question: &str,
        context: Option<String>,
    ) -> Result<Vec<ChatCompletionRequestMessage>> {
        let mut messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content("你是一个有用的助手，用中文回答问题。")
                .build()?
                .into(),
        ];

        // 注入检索到的上下文
        if let Some(ctx) = context {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(&format!("以下是相关的历史对话上下文：\n{}", ctx))
                    .build()?
                    .into(),
            );
        }

        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(question)
                .build()?
                .into(),
        );

        Ok(messages)
    }
}
```

## 模块依赖关系

```
┌─────────────────────────────────────────────────────────────┐
│                      bot-runtime                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              MemoryMiddleware                        │    │
│  └─────────────────────────────────────────────────────┘    │
└──────────────────────┬──────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                   telegram-bot-ai                           │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              TelegramBotAI                           │    │
│  └─────────────────────────────────────────────────────┘    │
└──────────────────────┬──────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                      memory                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              MemoryStore (trait)                     │    │
│  │  ┌─────────────────────────────────────────────┐   │    │
│  │  │         InMemoryVectorStore                  │   │    │
│  │  └─────────────────────────────────────────────┘   │    │
│  │  ┌─────────────────────────────────────────────┐   │    │
│  │  │         LanceVectorStore                    │   │    │
│  │  └─────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              EmbeddingService                       │    │
│  │              OpenAIEmbedding                        │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                      storage                                 │
│              MessageRepository (SQLite)                        │
└─────────────────────────────────────────────────────────────┘
```
