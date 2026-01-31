# Config 重构详细方案：可扩展的配置架构

## 1. 目标与原则

### 1.1 核心目标
- **telegram-bot** 仅保留：Telegram Bot 相关 + 日志 + 数据库 等基础配置
- **可扩展**：开发者可注入 LLM、Memory、Embedding 及自定义业务配置
- **解耦**：领域配置归属各自 crate，职责单一

### 1.2 设计原则
- **向后兼容**：`BotConfig::load(token)` 行为不变，现有调用方无需改动
- **渐进迁移**：分阶段实施，每阶段可独立验证
- **类型安全**：保持强类型，避免 `HashMap<String, String>` 的运行时错误
- **职责归属**：非 Telegram 配置（LLM、Memory、Embedding）必须在各自 crate 内，telegram-bot 只做组合与编排

---

## 2. 现状分析

### 2.1 当前 BotConfig 字段完整清单

| 字段 | 类型 | 环境变量 | 默认值 | 使用者 |
|------|------|----------|--------|--------|
| `bot_token` | `String` | `BOT_TOKEN` | 必填 | runner, components |
| `telegram_api_url` | `Option<String>` | `TELEGRAM_API_URL` / `TELOXIDE_API_URL` | None | components |
| `telegram_edit_interval_secs` | `u64` | `TELEGRAM_EDIT_INTERVAL_SECS` | 5 | components → SyncLLMHandler |
| `log_file` | `String` | 硬编码 | `logs/telegram-bot.log` | runner |
| `database_url` | `String` | `DATABASE_URL` | `file:./telegram_bot.db` | components |
| `openai_api_key` | `String` | `OPENAI_API_KEY` | 必填 | components |
| `openai_base_url` | `String` | `OPENAI_BASE_URL` | `https://api.openai.com/v1` | components |
| `llm_model` | `String` | `MODEL` | `gpt-3.5-turbo` | components |
| `llm_use_streaming` | `bool` | `USE_STREAMING` | false | components |
| `llm_thinking_message` | `String` | `THINKING_MESSAGE` | `Thinking...` | components |
| `llm_system_prompt` | `Option<String>` | `LLM_SYSTEM_PROMPT` | None | components |
| `memory_store_type` | `String` | `MEMORY_STORE_TYPE` | `memory` | components |
| `memory_sqlite_path` | `String` | `MEMORY_SQLITE_PATH` | `./data/memory.db` | components |
| `memory_recent_use_sqlite` | `bool` | `MEMORY_RECENT_USE_SQLITE` | false | components |
| `memory_lance_path` | `Option<String>` | `MEMORY_LANCE_PATH` / `LANCE_DB_PATH` | None | components |
| `memory_recent_limit` | `u32` | `MEMORY_RECENT_LIMIT` | 10 | components → SyncLLMHandler |
| `memory_relevant_top_k` | `u32` | `MEMORY_RELEVANT_TOP_K` | 5 | components → SyncLLMHandler |
| `memory_semantic_min_score` | `f32` | `MEMORY_SEMANTIC_MIN_SCORE` | 0.0 | components → SyncLLMHandler |
| `embedding_provider` | `String` | `EMBEDDING_PROVIDER` | `openai` | components |
| `bigmodel_api_key` | `String` | `BIGMODEL_API_KEY` / `ZHIPUAI_API_KEY` | 空 | components |

### 2.2 字段归属划分

| 归属 | 字段 | 理由 |
|------|------|------|
| **BaseConfig** | `bot_token`, `telegram_api_url`, `telegram_edit_interval_secs`, `log_file`, `database_url` | 与 Telegram 连接、日志、持久化直接相关 |
| **LlmConfig** | `openai_api_key`, `openai_base_url`, `llm_model`, `llm_use_streaming`, `llm_thinking_message`, `llm_system_prompt` | 纯 LLM 客户端配置 |
| **MemoryConfig** | `memory_store_type`, `memory_sqlite_path`, `memory_recent_use_sqlite`, `memory_lance_path`, `memory_recent_limit`, `memory_relevant_top_k`, `memory_semantic_min_score` | 记忆存储与 RAG 策略参数 |
| **EmbeddingConfig** | `embedding_provider`, `bigmodel_api_key` | 向量化服务配置 |

### 2.3 当前 validate() 逻辑
- `embedding_provider == "zhipuai"` 时需 `bigmodel_api_key` 非空
- `telegram_api_url` 若设置则须为合法 URL

---

## 3. 目标架构

### 3.1 模块依赖关系

**重要约束**：非 Telegram 相关的配置（LlmConfig、MemoryConfig、EmbeddingConfig）必须放在各自所属的 crate 内，**不得**放在 telegram-bot crate 中。

```
                    ┌─────────────────────────────────────────┐
                    │  telegram-bot                            │
                    │  ├── config/mod.rs (BotConfig)           │
                    │  ├── config/base.rs (BaseConfig)         │
                    │  └── config/extensions.rs                │
                    │       ├── AppExtensions trait            │
                    │       └── DefaultAppExtensions（组合各crate的Config）│
                    └──────────────┬──────────────────────────┘
                                   │ 依赖并引用
         ┌─────────────────────────┼─────────────────────────┐
         │                         │                         │
         ▼                         ▼                         ▼
┌─────────────────┐    ┌─────────────────────┐    ┌──────────────────┐
│ memory/         │    │ llm-client/         │    │ embedding/       │
│ src/config.rs   │    │ src/config.rs       │    │ src/config.rs    │
│ MemoryConfig    │    │ LlmConfig           │    │ EmbeddingConfig  │
│ EnvMemoryConfig │    │ EnvLlmConfig        │    │ EnvEmbeddingConfig│
└─────────────────┘    └─────────────────────┘    └──────────────────┘
```

### 3.2 目录结构

```
llm-client/src/
├── lib.rs
├── config.rs             # LlmConfig trait + EnvLlmConfig（新建）
└── openai_llm.rs

memory/src/
├── lib.rs
├── config.rs             # MemoryConfig trait + EnvMemoryConfig（新建）
├── context/
└── ...

crates/embedding/embedding/src/
├── lib.rs
└── config.rs             # EmbeddingConfig trait + EnvEmbeddingConfig（新建）

telegram-bot/src/
├── config/
│   ├── mod.rs            # 导出 BotConfig, BaseConfig, AppExtensions, DefaultAppExtensions
│   ├── base.rs           # BaseConfig（仅 Telegram + 日志 + 数据库）
│   └── extensions.rs     # AppExtensions trait + DefaultAppExtensions（引用 llm_client/memory/embedding 的 Config）
├── components.rs
├── runner.rs
└── ...
```

**telegram-bot 的 config 模块不包含** llm.rs、memory.rs、embedding.rs。

**依赖方向**：telegram-bot 依赖 llm-client、memory、embedding；后者不依赖 telegram-bot，避免循环依赖。

---

## 4. 接口与实现详细设计

### 4.1 BaseConfig（完整代码骨架）

```rust
// config/base.rs
//! 基础配置：Telegram Bot 连接、日志、数据库。从环境变量加载。

use anyhow::Result;
use std::env;

/// 基础配置：仅包含 Telegram 相关、日志、数据库。
#[derive(Debug, Clone)]
pub struct BaseConfig {
    /// BOT_TOKEN
    pub bot_token: String,
    /// TELEGRAM_API_URL 或 TELOXIDE_API_URL
    pub telegram_api_url: Option<String>,
    /// 流式回复时消息编辑最小间隔（秒），限制 Telegram API 调用频率
    pub telegram_edit_interval_secs: u64,
    /// 日志文件路径
    pub log_file: String,
    /// 消息持久化数据库 URL（SQLite file: 或 PostgreSQL 等）
    pub database_url: String,
}

impl BaseConfig {
    /// 从环境变量加载。`token` 若提供则覆盖 BOT_TOKEN。
    pub fn load(token: Option<String>) -> Result<Self> {
        let bot_token = token
            .unwrap_or_else(|| env::var("BOT_TOKEN").expect("BOT_TOKEN not set"));
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "file:./telegram_bot.db".to_string());
        let log_file = env::var("LOG_FILE")
            .unwrap_or_else(|_| "logs/telegram-bot.log".to_string());
        let telegram_api_url = env::var("TELEGRAM_API_URL")
            .or_else(|_| env::var("TELOXIDE_API_URL"))
            .ok();
        let telegram_edit_interval_secs = env::var("TELEGRAM_EDIT_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        Ok(Self {
            bot_token,
            telegram_api_url,
            telegram_edit_interval_secs,
            log_file,
            database_url,
        })
    }

    /// 校验配置合法性
    pub fn validate(&self) -> Result<()> {
        if let Some(ref url_str) = self.telegram_api_url {
            if reqwest::Url::parse(url_str).is_err() {
                anyhow::bail!(
                    "TELEGRAM_API_URL (or TELOXIDE_API_URL) is set but not a valid URL: {}",
                    url_str
                );
            }
        }
        Ok(())
    }
}
```

**变更说明**：
- `log_file` 支持 `LOG_FILE` 环境变量覆盖默认值
- `validate` 中移除 embedding 相关校验（移至 EmbeddingConfig）

---

### 4.2 LlmConfig 接口与实现（llm-client crate）

```rust
// llm-client/src/config.rs
/// LLM 配置接口。OpenAI 兼容 API 的通用参数。
pub trait LlmConfig: Send + Sync {
    fn api_key(&self) -> &str;
    fn base_url(&self) -> &str;
    fn model(&self) -> &str;
    fn use_streaming(&self) -> bool;
    fn thinking_message(&self) -> &str;
    fn system_prompt(&self) -> Option<&str>;
}

/// 从环境变量加载的 LLM 配置实现
#[derive(Debug, Clone)]
pub struct EnvLlmConfig {
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub llm_model: String,
    pub llm_use_streaming: bool,
    pub llm_thinking_message: String,
    pub llm_system_prompt: Option<String>,
}

impl LlmConfig for EnvLlmConfig {
    fn api_key(&self) -> &str { &self.openai_api_key }
    fn base_url(&self) -> &str { &self.openai_base_url }
    fn model(&self) -> &str { &self.llm_model }
    fn use_streaming(&self) -> bool { self.llm_use_streaming }
    fn thinking_message(&self) -> &str { &self.llm_thinking_message }
    fn system_prompt(&self) -> Option<&str> { self.llm_system_prompt.as_deref() }
}

impl EnvLlmConfig {
    pub fn from_env() -> Result<Self> {
        let openai_api_key = env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
        let openai_base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let llm_model = env::var("MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let llm_use_streaming = env::var("USE_STREAMING")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);
        let llm_thinking_message = env::var("THINKING_MESSAGE")
            .unwrap_or_else(|_| "Thinking...".to_string());
        let llm_system_prompt = env::var("LLM_SYSTEM_PROMPT")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Ok(Self {
            openai_api_key,
            openai_base_url,
            llm_model,
            llm_use_streaming,
            llm_thinking_message,
            llm_system_prompt,
        })
    }
}
```

---

### 4.3 MemoryConfig 接口与实现（memory crate）

```rust
// memory/src/config.rs
/// Memory 存储与 RAG 策略配置接口
pub trait MemoryConfig: Send + Sync {
    fn store_type(&self) -> &str;
    fn sqlite_path(&self) -> &str;
    fn recent_use_sqlite(&self) -> bool;
    fn lance_path(&self) -> Option<&str>;
    fn recent_limit(&self) -> u32;
    fn relevant_top_k(&self) -> u32;
    fn semantic_min_score(&self) -> f32;
}

#[derive(Debug, Clone)]
pub struct EnvMemoryConfig {
    pub memory_store_type: String,
    pub memory_sqlite_path: String,
    pub memory_recent_use_sqlite: bool,
    pub memory_lance_path: Option<String>,
    pub memory_recent_limit: u32,
    pub memory_relevant_top_k: u32,
    pub memory_semantic_min_score: f32,
}

impl MemoryConfig for EnvMemoryConfig { /* ... */ }

impl EnvMemoryConfig {
    pub fn from_env() -> Result<Self> {
        let memory_store_type = env::var("MEMORY_STORE_TYPE")
            .unwrap_or_else(|_| "memory".to_string());
        let memory_sqlite_path = env::var("MEMORY_SQLITE_PATH")
            .unwrap_or_else(|_| "./data/memory.db".to_string());
        let memory_recent_use_sqlite = env::var("MEMORY_RECENT_USE_SQLITE")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "1" | "true" | "yes" => Some(true),
                _ => s.parse().ok(),
            })
            .unwrap_or(false);
        let memory_lance_path = env::var("MEMORY_LANCE_PATH")
            .or_else(|_| env::var("LANCE_DB_PATH"))
            .ok();
        let memory_recent_limit = env::var("MEMORY_RECENT_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        let memory_relevant_top_k = env::var("MEMORY_RELEVANT_TOP_K")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let memory_semantic_min_score = env::var("MEMORY_SEMANTIC_MIN_SCORE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        Ok(Self { /* ... */ })
    }
}
```

---

### 4.4 EmbeddingConfig 接口与实现（embedding crate）

```rust
// crates/embedding/embedding/src/config.rs
/// Embedding 服务配置接口
pub trait EmbeddingConfig: Send + Sync {
    fn provider(&self) -> &str;
    fn bigmodel_api_key(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct EnvEmbeddingConfig {
    pub embedding_provider: String,
    pub bigmodel_api_key: String,
}

impl EmbeddingConfig for EnvEmbeddingConfig {
    fn provider(&self) -> &str { &self.embedding_provider }
    fn bigmodel_api_key(&self) -> &str { &self.bigmodel_api_key }
}

impl EnvEmbeddingConfig {
    pub fn from_env() -> Result<Self> {
        let embedding_provider = env::var("EMBEDDING_PROVIDER")
            .unwrap_or_else(|_| "openai".to_string());
        let bigmodel_api_key = env::var("BIGMODEL_API_KEY")
            .or_else(|_| env::var("ZHIPUAI_API_KEY"))
            .unwrap_or_default();
        Ok(Self { embedding_provider, bigmodel_api_key })
    }

    pub fn validate(&self) -> Result<()> {
        if self.embedding_provider.eq_ignore_ascii_case("zhipuai") && self.bigmodel_api_key.is_empty() {
            anyhow::bail!(
                "EMBEDDING_PROVIDER=zhipuai requires BIGMODEL_API_KEY or ZHIPUAI_API_KEY"
            );
        }
        Ok(())
    }
}
```

---

### 4.5 AppExtensions trait 与 DefaultAppExtensions（telegram-bot crate）

telegram-bot 仅定义扩展 trait 和默认实现，**不定义** LlmConfig、MemoryConfig、EmbeddingConfig，而是引用各 crate 的导出。

```rust
// telegram-bot/src/config/extensions.rs
use llm_client::{LlmConfig, EnvLlmConfig};
use memory::{MemoryConfig, EnvMemoryConfig};
use embedding::{EmbeddingConfig, EnvEmbeddingConfig};

/// 应用扩展配置。开发者实现此 trait 以注入自定义配置。
pub trait AppExtensions: Send + Sync {
    fn llm_config(&self) -> Option<&dyn LlmConfig>;
    fn memory_config(&self) -> Option<&dyn MemoryConfig>;
    fn embedding_config(&self) -> Option<&dyn EmbeddingConfig>;

    /// 自定义扩展：供自定义 handler 通过类型获取
    fn get<T: 'static>(&self) -> Option<&T> {
        None
    }
}

/// 默认扩展：组合 llm-client、memory、embedding 各 crate 的 Env*Config
pub struct DefaultAppExtensions {
    pub llm: EnvLlmConfig,
    pub memory: EnvMemoryConfig,
    pub embedding: EnvEmbeddingConfig,
}

impl AppExtensions for DefaultAppExtensions {
    fn llm_config(&self) -> Option<&dyn LlmConfig> { Some(&self.llm) }
    fn memory_config(&self) -> Option<&dyn MemoryConfig> { Some(&self.memory) }
    fn embedding_config(&self) -> Option<&dyn EmbeddingConfig> { Some(&self.embedding) }
}

impl DefaultAppExtensions {
    pub fn from_env() -> Result<Self> {
        let llm = EnvLlmConfig::from_env()?;
        let memory = EnvMemoryConfig::from_env()?;
        let embedding = EnvEmbeddingConfig::from_env()?;
        embedding.validate()?;
        Ok(Self { llm, memory, embedding })
    }
}
```

---

### 4.6 BotConfig 组合

```rust
// config/mod.rs
pub struct BotConfig<E = DefaultAppExtensions>
where
    E: AppExtensions,
{
    pub base: BaseConfig,
    pub extensions: E,
}

impl<E: AppExtensions> BotConfig<E> {
    pub fn base(&self) -> &BaseConfig { &self.base }
    pub fn extensions(&self) -> &E { &self.extensions }
}

impl BotConfig<DefaultAppExtensions> {
    /// 便捷方法：从环境变量加载完整配置（BaseConfig + DefaultAppExtensions）
    pub fn load(token: Option<String>) -> Result<Self> {
        let base = BaseConfig::load(token)?;
        let extensions = DefaultAppExtensions::from_env()?;
        base.validate()?;
        Ok(Self { base, extensions })
    }
}
```

---

## 5. components.rs 重构前后对比

### 5.1 create_memory_stores

**当前**：
```rust
pub async fn create_memory_stores(config: &BotConfig) -> Result<...> {
    let memory_store = match config.memory_store_type.as_str() {
        "lance" => { /* 使用 config.memory_lance_path, config.embedding_provider */ }
        "sqlite" => { /* 使用 config.memory_sqlite_path */ }
        _ => { /* in-memory */ }
    };
    let recent_store = if config.memory_recent_use_sqlite { ... } else { None };
    ...
}
```

**重构后**：
```rust
pub async fn create_memory_stores<E: AppExtensions>(config: &BotConfig<E>) -> Result<...> {
    let mem_cfg = config.extensions().memory_config()
        .ok_or_else(|| anyhow!("MemoryConfig required for default component assembly"))?;
    let emb_cfg = config.extensions().embedding_config();

    let memory_store = match mem_cfg.store_type() {
        "lance" => {
            let lance_path = mem_cfg.lance_path().unwrap_or("./data/lance_db");
            let embedding_dim = emb_cfg
                .map(|e| if e.provider().eq_ignore_ascii_case("zhipuai") { 1024 } else { 1536 })
                .unwrap_or(1536);
            // ...
        }
        "sqlite" => { /* mem_cfg.sqlite_path() */ }
        _ => { /* InMemoryVectorStore::new() */ }
    };
    let recent_store = if mem_cfg.recent_use_sqlite() { ... } else { None };
    ...
}
```

### 5.2 build_bot_components

**当前**：直接访问 `config.database_url`, `config.bot_token`, `config.openai_api_key` 等。

**重构后**：
- `config.base.database_url`, `config.base.bot_token`, `config.base.telegram_api_url`, `config.base.telegram_edit_interval_secs`
- `config.extensions().llm_config()?.api_key()`, `model()`, `use_streaming()` 等
- `config.extensions().memory_config()?.recent_limit()`, `relevant_top_k()`, `semantic_min_score()`
- `config.extensions().embedding_config()?.provider()`, `bigmodel_api_key()`

### 5.3 函数签名变更

```rust
// 旧
pub async fn initialize_bot_components(config: &BotConfig) -> Result<BotComponents>;

// 新（泛型）
pub async fn initialize_bot_components<E: AppExtensions>(
    config: &BotConfig<E>,
) -> Result<BotComponents>;
```

---

## 6. runner.rs 重构

### 6.1 run_bot 变更

**当前**：
```rust
pub async fn run_bot(config: BotConfig) -> Result<()> {
    config.validate()?;
    std::fs::create_dir_all("logs").expect("...");
    init_tracing(&config.log_file)?;
    info!(database_url = %config.database_url, ...);
    let bot = TelegramBot::new(config).await?;
    ...
}
```

**重构后**：
```rust
pub async fn run_bot<E: AppExtensions>(config: BotConfig<E>) -> Result<()> {
    config.base().validate()?;
    let log_dir = std::path::Path::new(config.base().log_file).parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(log_dir).expect("...");
    init_tracing(config.base().log_file)?;
    info!(database_url = %config.base().database_url, ...);
    let bot = TelegramBot::new(config).await?;
    ...
}
```

### 6.2 TelegramBot 结构体

```rust
pub struct TelegramBot<E = DefaultAppExtensions>
where
    E: AppExtensions,
{
    pub config: BotConfig<E>,
    pub components: BotComponents,
    pub handler_chain: HandlerChain,
}
```

---

## 7. 迁移步骤（分阶段）

### Phase 1：在各 crate 内新建 Config（1 天）

1. **llm-client**：新建 `llm-client/src/config.rs`，定义 `LlmConfig` trait 与 `EnvLlmConfig`，实现 `from_env()`
2. **memory**：新建 `memory/src/config.rs`，定义 `MemoryConfig` trait 与 `EnvMemoryConfig`，实现 `from_env()`
3. **embedding**：新建 `crates/embedding/embedding/src/config.rs`，定义 `EmbeddingConfig` trait 与 `EnvEmbeddingConfig`，实现 `from_env()` 和 `validate()`
4. 各 crate 在 `lib.rs` 中导出 `pub mod config` 及 `pub use config::{*Config, Env*Config}`
5. **llm-client** 需添加 `std::env` 使用（无新依赖）；**memory**、**embedding** 已有 `anyhow`，可满足 `from_env` 返回类型

**验收**：`cargo test -p llm-client`、`cargo test -p memory`、`cargo test -p embedding` 通过（各 crate 内 config 单测）

---

### Phase 2：telegram-bot 拆分 BaseConfig 与 extensions（1–2 天）

1. **新建** `telegram-bot/src/config/` 目录
2. **新建** `config/base.rs`，实现 `BaseConfig` 及 `load`/`validate`
3. **新建** `config/extensions.rs`，定义 `AppExtensions` trait 和 `DefaultAppExtensions`（依赖 `llm_client`、`memory`、`embedding` 的 Config）
4. **修改** `config/mod.rs`：组合 `BotConfig` = `BaseConfig` + `DefaultAppExtensions`
5. **修改** `BotConfig::load`：调用 `BaseConfig::load` + `DefaultAppExtensions::from_env`
6. **修改** `telegram-bot/Cargo.toml`：确保依赖 `llm-client`、`memory`、`embedding`（已有）
7. **临时** 在 `BotConfig` 上保留兼容 getter，使 `components.rs`、`runner.rs` 仍能通过 `config.xxx` 访问

**验收**：`cargo test -p telegram-bot config` 全部通过，`dbot run` 能正常启动

---

### Phase 3：components 改为使用 extensions（1 天）

1. **修改** `create_memory_stores`：从 `config.extensions.memory_config()`、`embedding_config()` 读取
2. **修改** `build_bot_components`：从 `config.base` 取 base 字段，从 `config.extensions` 取 LLM/Memory/Embedding
3. **移除** `BotConfig` 上的兼容 getter
4. **运行** 集成测试

**验收**：`cargo test -p telegram-bot` 和 `telegram-bot/tests/runner_integration_test` 通过

---

### Phase 4：泛型化与自定义扩展（1 天）

1. **将** `BotConfig` 改为 `BotConfig<E: AppExtensions>`
2. **将** `TelegramBot`、`run_bot`、`initialize_bot_components` 等改为泛型 `E: AppExtensions`
3. **为** `BotConfig` 实现 `BotConfig<DefaultAppExtensions>` 的 `load` 便捷方法
4. **编写** 示例：自定义 `MyAppExtensions` 实现 `AppExtensions`，从 YAML 或自定义逻辑加载

**验收**：示例能跑通，`dbot-cli` 无需改动

---

## 8. 测试策略

### 8.1 各 crate 内的 Config 单测
- **llm-client**：`test_env_llm_config_from_env`、`test_env_llm_config_defaults`
- **memory**：`test_env_memory_config_from_env`、`test_env_memory_config_defaults`
- **embedding**：`test_env_embedding_config_from_env`、`test_env_embedding_validate_zhipuai_requires_key`

各 crate 的 config 单测沿用现有 `BotConfig` 单测中的 env 设置逻辑，断言 `from_env()` 返回值。

### 8.2 BaseConfig 单测（telegram-bot）
- `test_base_config_load_defaults`
- `test_base_config_load_custom`
- `test_base_config_validate_telegram_url_invalid`
- `test_base_config_validate_ok`

### 8.3 BotConfig::load 兼容性测试
- 保持现有 `test_load_config_with_defaults` 等，断言 `config.base()` 与 `config.extensions().llm_config()` 等字段值
- 确保 `BotConfig::load(None)` 行为与当前一致

### 8.4 集成测试
- `runner_integration_test` 中 `setup_test_config` 继续使用 `BotConfig::load(None)`，环境变量设置方式不变

---

## 9. 环境变量速查表（重构后）

| 配置 | 环境变量 | 默认 |
|------|----------|------|
| BaseConfig | BOT_TOKEN | 必填 |
| | TELEGRAM_API_URL / TELOXIDE_API_URL | None |
| | TELEGRAM_EDIT_INTERVAL_SECS | 5 |
| | LOG_FILE | logs/telegram-bot.log |
| | DATABASE_URL | file:./telegram_bot.db |
| LlmConfig | OPENAI_API_KEY | 必填 |
| | OPENAI_BASE_URL | https://api.openai.com/v1 |
| | MODEL | gpt-3.5-turbo |
| | USE_STREAMING | false |
| | THINKING_MESSAGE | Thinking... |
| | LLM_SYSTEM_PROMPT | None |
| MemoryConfig | MEMORY_STORE_TYPE | memory |
| | MEMORY_SQLITE_PATH | ./data/memory.db |
| | MEMORY_RECENT_USE_SQLITE | false |
| | MEMORY_LANCE_PATH / LANCE_DB_PATH | None |
| | MEMORY_RECENT_LIMIT | 10 |
| | MEMORY_RELEVANT_TOP_K | 5 |
| | MEMORY_SEMANTIC_MIN_SCORE | 0.0 |
| EmbeddingConfig | EMBEDDING_PROVIDER | openai |
| | BIGMODEL_API_KEY / ZHIPUAI_API_KEY | 空 |

---

## 10. 自定义扩展示例

### 10.1 从 TOML 加载的 LLM 配置（开发者自定义，实现 llm_client::LlmConfig）

```rust
use llm_client::LlmConfig;
use std::fs;

struct TomlLlmConfig {
    api_key: String,
    base_url: String,
    model: String,
    use_streaming: bool,
    thinking_message: String,
    system_prompt: Option<String>,
}

impl LlmConfig for TomlLlmConfig { /* ... */ }

impl TomlLlmConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let s = fs::read_to_string(path)?;
        let t: toml::Value = toml::from_str(&s)?;
        Ok(Self {
            api_key: t["openai"]["api_key"].as_str().unwrap().to_string(),
            base_url: t["openai"]["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string(),
            // ...
        })
    }
}
```

### 10.2 自定义 AppExtensions

```rust
use llm_client::{LlmConfig, EnvLlmConfig};
use memory::{MemoryConfig, EnvMemoryConfig};
use embedding::{EmbeddingConfig, EnvEmbeddingConfig};

struct MyExtensions {
    llm: TomlLlmConfig,
    memory: EnvMemoryConfig,   // 来自 memory crate
    embedding: EnvEmbeddingConfig,  // 来自 embedding crate
    business: MyBusinessConfig,
}

impl AppExtensions for MyExtensions {
    fn llm_config(&self) -> Option<&dyn LlmConfig> { Some(&self.llm) }
    fn memory_config(&self) -> Option<&dyn MemoryConfig> { Some(&self.memory) }
    fn embedding_config(&self) -> Option<&dyn EmbeddingConfig> { Some(&self.embedding) }
    fn get<T: 'static>(&self) -> Option<&T> {
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<MyBusinessConfig>() {
            return (self as &dyn std::any::Any).downcast_ref();
        }
        None
    }
}

// 使用
let base = BaseConfig::load(None)?;
let ext = MyExtensions {
    llm: TomlLlmConfig::from_file("config/llm.toml")?,
    memory: EnvMemoryConfig::from_env()?,
    embedding: EnvEmbeddingConfig::from_env()?,
    business: MyBusinessConfig::new(),
};
let config = BotConfig { base, extensions: ext };
run_bot(config).await?;
```

---

## 11. 回滚计划

- 每阶段在独立分支完成，通过 CI 后合并
- Phase 1 结束时保留旧 `BotConfig` 的兼容 getter，若 Phase 2 出问题可快速回退
- 若需完全回退，可从 `main` 恢复 `telegram-bot/src/config.rs` 单文件，并还原 `components`、`runner` 的修改

---

## 12. 文件变更清单

| 文件 | 操作 |
|------|------|
| **llm-client** | |
| `llm-client/src/config.rs` | 新建：LlmConfig trait + EnvLlmConfig |
| `llm-client/src/lib.rs` | 修改：`pub mod config`，`pub use config::*` |
| **memory** | |
| `memory/src/config.rs` | 新建：MemoryConfig trait + EnvMemoryConfig |
| `memory/src/lib.rs` | 修改：`pub mod config`，`pub use config::*` |
| **embedding** | |
| `crates/embedding/embedding/src/config.rs` | 新建：EmbeddingConfig trait + EnvEmbeddingConfig |
| `crates/embedding/embedding/src/lib.rs` | 修改：`pub mod config`，`pub use config::*` |
| **telegram-bot** | |
| `telegram-bot/src/config.rs` | 删除，拆分为 config/ 模块 |
| `telegram-bot/src/config/mod.rs` | 新建 |
| `telegram-bot/src/config/base.rs` | 新建（仅 BaseConfig） |
| `telegram-bot/src/config/extensions.rs` | 新建（AppExtensions + DefaultAppExtensions，引用各 crate 的 Config） |
| `telegram-bot/src/components.rs` | 修改：从 extensions 读取配置 |
| `telegram-bot/src/runner.rs` | 修改：使用 config.base |
| `telegram-bot/src/lib.rs` | 修改：导出 config 子模块 |
| **其他** | |
| `dbot-cli/src/main.rs` | 无需修改 |
| `telegram-bot/tests/runner_integration_test.rs` | 无需修改 |

---

## 13. 预计工时

| 阶段 | 内容 | 预估 |
|------|------|------|
| Phase 1 | 在 llm-client、memory、embedding 各 crate 内新建 Config | 1 天 |
| Phase 2 | telegram-bot 拆分 BaseConfig、extensions、兼容层 | 1–2 天 |
| Phase 3 | components 迁移到 extensions | 1 天 |
| Phase 4 | 泛型化、自定义扩展示例 | 1 天 |
| 测试与文档 | 单测迁移、README 更新 | 0.5 天 |

**合计**：约 4–5 天
