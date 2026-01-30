# Memory Loader

将 SQLite 消息数据加载到 LanceDB 向量数据库。

## 概述

提供 `load` 功能，从 SQLite 读取消息，生成 embedding，写入 LanceDB。由 `dbot-cli` 调用。

## 命令行接口（通过 dbot-cli）

```bash
# 基本用法
./target/release/dbot load

# 指定批量大小
./target/release/dbot load --batch-size 100
```

## 开发计划

| 序号 | 任务 | 描述 | 状态 |
|------|------|------|------|
| 1 | LoadConfig 结构体 | 定义配置：database_url, lance_db_path, openai_api_key, batch_size | 待开始 |
| 2 | LoadResult 结构体 | 定义结果：total, loaded, elapsed_secs | 待开始 |
| 3 | convert 函数 | MessageRecord → MemoryEntry 转换 | 待开始 |
| 4 | load 函数 | 核心加载逻辑：读取 → 转换 → embedding → 写入 | 待开始 |
| 5 | 单元测试 | convert 函数测试 | 待开始 |
| 6 | CLI 集成 | dbot-cli 添加 load 子命令 | 待开始 |

## 技术设计

### 1. 公开接口

```rust
/// 词向量服务：OpenAI / Zhipuai
pub enum EmbeddingProvider { OpenAI, Zhipuai }

/// 词向量配置（可从 .env 构造）
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub model: Option<String>,
    pub openai_api_key: String,
    pub bigmodel_api_key: String,
}

/// 数据加载配置
pub struct LoadConfig {
    pub database_url: String,
    pub lance_db_path: String,
    pub embedding: EmbeddingConfig,
    pub batch_size: usize,
}

/// 加载结果
pub struct LoadResult {
    pub total: usize,
    pub loaded: usize,
    pub elapsed_secs: u64,
}

/// 执行数据加载
/// 从 SQLite 读取消息，生成 embedding，写入 LanceDB
pub async fn load(config: LoadConfig) -> Result<LoadResult>
```

### 2. 数据转换

```rust
/// MessageRecord → MemoryEntry 字段映射
/// - id: 保留原始 UUID
/// - content: message.content
/// - metadata.user_id: message.user_id (String)
/// - metadata.conversation_id: message.chat_id (String)
/// - metadata.role: direction="received" → User, "sent" → Assistant
/// - metadata.timestamp: message.created_at
fn convert(msg: &MessageRecord) -> MemoryEntry
```

### 3. 处理流程

```
1. 连接 SQLite (MessageRepository)
2. 连接 LanceDB (LanceVectorStore)
3. 初始化 EmbeddingService (OpenAIEmbedding)
4. 获取消息总数
5. 批量循环:
   - 读取 batch_size 条消息
   - 转换为 MemoryEntry
   - 调用 embed_batch 生成向量
   - 写入 LanceDB
6. 返回 LoadResult
```

### 4. CLI 集成（dbot-cli）

```rust
// dbot-cli/src/main.rs
use memory_loader::{load, LoadConfig};

#[derive(Subcommand)]
enum Commands {
    Run { ... },
    Load {
        #[arg(short, long, default_value = "50")]
        batch_size: usize,
    },
}

async fn handle_load(batch_size: usize) -> Result<()> {
    let config = LoadConfig {
        database_url: std::env::var("DATABASE_URL")?,
        lance_db_path: std::env::var("LANCE_DB_PATH")?,
        openai_api_key: std::env::var("OPENAI_API_KEY")?,
        batch_size,
    };
    let result = load(config).await?;
    println!("Total: {}, Loaded: {}, Time: {}s", 
        result.total, result.loaded, result.elapsed_secs);
    Ok(())
}
```

## 环境变量

从 `.env` 文件读取（使用 dotenvy）：

```env
DATABASE_URL=file:./telegram_bot.db   # SQLite 路径
LANCE_DB_PATH=./lancedb               # LanceDB 路径

# 词向量（二选一）
EMBEDDING_PROVIDER=openai              # openai / zhipuai
OPENAI_API_KEY=sk-xxx                  # provider=openai 时必填
# BIGMODEL_API_KEY=xxx                 # provider=zhipuai 时必填
# EMBEDDING_MODEL=text-embedding-3-small  # 可选，默认见上表
```

## 依赖

```toml
[dependencies]
storage = { path = "../../storage" }
memory-lance = { path = "../memory-lance" }
memory-core = { path = "../memory-core" }
embedding = { path = "../embedding" }
openai-embedding = { path = "../openai-embedding" }
```

---

## 扩展方案：支持 .env 中的智谱/GLM 词向量

### 目标

- `dbot load` 做词向量时，从 `.env` 读取 embedding 相关配置。
- 支持选择 **OpenAI** 或 **智谱 BigModel** 作为 embedding 服务。
- 支持从 `.env` 指定 **embedding 模型名**（如智谱 `embedding-2`），与现有 `AI_MODEL`（对话模型）区分。

### 环境变量设计

| 变量 | 说明 | 默认 | 示例 |
|------|------|------|------|
| `EMBEDDING_PROVIDER` | 词向量服务：`openai` / `zhipuai`（智谱） | `openai` | `zhipuai` |
| `OPENAI_API_KEY` | OpenAI API Key（provider=openai 时必填） | - | `sk-xxx` |
| `BIGMODEL_API_KEY` | 智谱 API Key（provider=zhipuai 时必填） | - | `xxx` |
| `EMBEDDING_MODEL` | 词向量模型名（可选，不填则用各 provider 默认） | openai: `text-embedding-3-small`<br>zhipuai: `embedding-2` | `embedding-2` |

说明：`AI_MODEL`（如 `glm-4-flash`）为**对话模型**，不参与 load 词向量；词向量仅由 `EMBEDDING_PROVIDER` + `EMBEDDING_MODEL`（及对应 API Key）决定。

### 开发计划

| 序号 | 任务 | 描述 | 涉及 | 状态 |
|------|------|------|------|------|
| 1 | EmbeddingProvider 枚举 | 定义 `OpenAI` / `Zhipuai`，解析 `EMBEDDING_PROVIDER` | memory-loader | 已完成 |
| 2 | LoadConfig 扩展 | 增加 `EmbeddingConfig`（provider、model、两个 key） | memory-loader | 已完成 |
| 3 | 按 provider 创建 EmbeddingService | 根据 config 创建 `OpenAIEmbedding` 或 `BigModelEmbedding`，传入 model | memory-loader | 已完成 |
| 4 | dbot-cli 读取 .env | 读 `EMBEDDING_PROVIDER`、`EMBEDDING_MODEL`、`OPENAI_API_KEY`、`BIGMODEL_API_KEY` 构造 LoadConfig | dbot-cli | 已完成 |
| 5 | 文档与默认值 | README / .env.example 补充上述变量说明 | memory-loader, dbot-cli | 已完成 |

### 技术设计

#### 1. memory-loader：EmbeddingProvider + 配置

```rust
/// 词向量服务提供商
pub enum EmbeddingProvider {
    OpenAI,
    Zhipuai,  // 智谱 BigModel
}

/// 词向量配置（可从 .env 构造）
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub model: Option<String>,   // None 表示用该 provider 默认
    pub openai_api_key: String,
    pub bigmodel_api_key: String, // 另一 provider 的 key 可为空
}

/// LoadConfig 扩展：保留原有字段，增加 embedding
pub struct LoadConfig {
    pub database_url: String,
    pub lance_db_path: String,
    pub batch_size: usize,
    pub embedding: EmbeddingConfig,
}
```

#### 2. memory-loader：按 provider 创建 EmbeddingService

```rust
fn create_embedding_service(config: &EmbeddingConfig) -> Arc<dyn EmbeddingService> {
    match config.provider {
        EmbeddingProvider::OpenAI => {
            let model = config.model.clone()
                .unwrap_or_else(|| "text-embedding-3-small".to_string());
            Arc::new(OpenAIEmbedding::new(config.openai_api_key.clone(), model))
        }
        EmbeddingProvider::Zhipuai => {
            let model = config.model.clone()
                .unwrap_or_else(|| "embedding-2".to_string());
            Arc::new(BigModelEmbedding::new(config.bigmodel_api_key.clone(), model))
        }
    }
}
```

#### 3. dbot-cli：从 .env 构造 LoadConfig

```rust
fn load_embedding_config() -> EmbeddingConfig {
    let provider = match std::env::var("EMBEDDING_PROVIDER").as_deref() {
        Ok("zhipuai") => EmbeddingProvider::Zhipuai,
        _ => EmbeddingProvider::OpenAI,
    };
    let model = std::env::var("EMBEDDING_MODEL").ok();
    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let bigmodel_api_key = std::env::var("BIGMODEL_API_KEY").unwrap_or_default();

    match &provider {
        EmbeddingProvider::OpenAI if openai_api_key.is_empty() => {
            panic!("OPENAI_API_KEY required when EMBEDDING_PROVIDER=openai");
        }
        EmbeddingProvider::Zhipuai if bigmodel_api_key.is_empty() => {
            panic!("BIGMODEL_API_KEY required when EMBEDDING_PROVIDER=zhipuai");
        }
        _ => {}
    }

    EmbeddingConfig {
        provider,
        model,
        openai_api_key,
        bigmodel_api_key,
    }
}

async fn handle_load(batch_size: usize) -> Result<()> {
    let config = LoadConfig {
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "file:./telegram_bot.db".to_string()),
        lance_db_path: std::env::var("LANCE_DB_PATH").unwrap_or_else(|_| "./lancedb".to_string()),
        batch_size,
        embedding: load_embedding_config(),
    };
    let result = load(config).await?;
    // ...
}
```

#### 4. 依赖（memory-loader）

```toml
# 新增
bigmodel-embedding = { path = "../bigmodel-embedding" }
```

### 使用示例（.env）

```env
# 使用智谱词向量（与现有 GLM 对话模型配套）
EMBEDDING_PROVIDER=zhipuai
BIGMODEL_API_KEY=your_bigmodel_key
EMBEDDING_MODEL=embedding-2

# 使用 OpenAI 词向量（默认）
# EMBEDDING_PROVIDER=openai
# OPENAI_API_KEY=sk-xxx
# EMBEDDING_MODEL=text-embedding-3-small
```

### 兼容性

- 不设置 `EMBEDDING_PROVIDER` 时，行为与当前一致：使用 OpenAI + `text-embedding-3-small`，仅需 `OPENAI_API_KEY`。
- 现有 `LoadConfig { openai_api_key, ... }` 可在一期保留为便捷构造，或废弃并统一走 `EmbeddingConfig`。
