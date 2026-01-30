use ai_handlers::SyncAIHandler;
use anyhow::Result;
use dbot_core::{init_tracing, Message as CoreMessage, ToCoreMessage};
use dbot_telegram::{run_repl, TelegramMessageWrapper};
use handler_chain::HandlerChain;
use memory::MemoryStore;
use memory_inmemory::InMemoryVectorStore;
use memory_lance::{LanceConfig, LanceVectorStore};
use memory_sqlite::SQLiteVectorStore;
use middleware::{MemoryMiddleware, PersistenceMiddleware};
use ai_client::OpenAILlmClient;
use bigmodel_embedding::BigModelEmbedding;
use dbot_telegram::TelegramBotAdapter;
use openai_embedding::OpenAIEmbedding;
use std::sync::Arc;
use storage::MessageRepository;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::config::BotConfig;

/// Bot 组件集合，封装 run_bot / TelegramBot 所需的核心依赖，便于测试与复用。
pub struct BotComponents {
    /// 消息持久化仓库
    pub repo: Arc<MessageRepository>,
    /// Teloxide Bot 实例
    pub teloxide_bot: Bot,
    /// Bot 的用户名缓存
    pub bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    /// 同步 AI 处理器（在链内执行，返回 Reply 供 middleware 存记忆）
    pub sync_ai_handler: Arc<SyncAIHandler>,
    /// 向量记忆存储（主存储，用于语义检索与默认读写）
    pub memory_store: Arc<dyn MemoryStore>,
    /// 可选：最近消息专用存储（如 SQLite）。设置时 RecentMessagesStrategy / UserPreferencesStrategy 从此读，middleware 同时写入此处与主存储。
    pub recent_store: Option<Arc<dyn MemoryStore>>,
    /// Embedding 服务（用于语义检索与写入记忆时生成向量）
    pub embedding_service: Arc<dyn embedding::EmbeddingService>,
}

/// 可测试的 TelegramBot 结构，封装 Bot 配置与依赖。
pub struct TelegramBot {
    /// 运行所需的配置
    pub config: BotConfig,
    /// 封装后的 Bot 组件
    pub components: BotComponents,
    /// 处理消息的中间件链
    pub handler_chain: HandlerChain,
}

/// 使用给定的 `MemoryStore` 构建 Bot 组件。
///
/// 注意：此函数假设传入的 `memory_store` 已根据配置选择好具体实现，
/// 因此不会再根据 `config.memory_store_type` 做分支判断。
/// 当 `recent_store` 为 Some 时，最近消息策略使用它，middleware 同时写入主存储与 recent_store。
#[instrument(skip(config, memory_store, recent_store))]
async fn build_bot_components(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
    recent_store: Option<Arc<dyn MemoryStore>>,
) -> Result<BotComponents> {
    // 初始化存储
    let repo = Arc::new(
        MessageRepository::new(&config.database_url)
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    database_url = %config.database_url,
                    "Failed to initialize message storage"
                );
                anyhow::anyhow!("Failed to initialize message storage: {}", e)
            })?,
    );

    // 初始化 Telegram Bot；若配置了 TELEGRAM_API_URL / TELOXIDE_API_URL（如测试 mock 服务器），则指向该 URL
    let teloxide_bot = {
        let bot = Bot::new(config.bot_token.clone());
        if let Some(ref url_str) = config.telegram_api_url {
            match reqwest::Url::parse(url_str) {
                Ok(url) => bot.set_api_url(url),
                Err(e) => {
                    error!(error = %e, url = %url_str, "Invalid TELEGRAM_API_URL, using default");
                    bot
                }
            }
        } else {
            bot
        }
    };

    // 存储 bot username
    let bot_username = Arc::new(tokio::sync::RwLock::new(None));

    // 初始化 LLM 客户端（ai-client）与 Bot 适配器（dbot-telegram）
    let llm_client = Arc::new(
        OpenAILlmClient::with_base_url(
            config.openai_api_key.clone(),
            config.openai_base_url.clone(),
        )
        .with_model(config.ai_model.clone())
        .with_system_prompt_opt(config.ai_system_prompt.clone()),
    );
    let bot_adapter: Arc<dyn dbot_core::Bot> =
        Arc::new(TelegramBotAdapter::new(teloxide_bot.clone()));

    // 初始化 Embedding 服务（用于用户提问在向量库中的语义搜索）
    let embedding_service: Arc<dyn embedding::EmbeddingService> = match config.embedding_provider.as_str() {
        "zhipuai" => {
            if config.bigmodel_api_key.is_empty() {
                error!("EMBEDDING_PROVIDER=zhipuai but BIGMODEL_API_KEY / ZHIPUAI_API_KEY not set");
                return Err(anyhow::anyhow!(
                    "BIGMODEL_API_KEY or ZHIPUAI_API_KEY required when EMBEDDING_PROVIDER=zhipuai"
                ));
            }
            info!("Using BigModel (Zhipu AI) embedding for RAG semantic search");
            Arc::new(BigModelEmbedding::with_api_key(config.bigmodel_api_key.clone()))
        }
        _ => {
            info!("Using OpenAI embedding for RAG semantic search");
            Arc::new(OpenAIEmbedding::with_api_key(config.openai_api_key.clone()))
        }
    };

    // 初始化同步 AI 处理器（在链内执行，返回 Reply 供 MemoryMiddleware 在 after() 中存记忆）
    // memory_recent_limit / memory_relevant_top_k 用于构造 ContextBuilder 的 RecentMessagesStrategy / SemanticSearchStrategy
    let sync_ai_handler = Arc::new(SyncAIHandler::new(
        bot_username.clone(),
        llm_client,
        bot_adapter,
        repo.as_ref().clone(),
        memory_store.clone(),
        recent_store.clone(),
        embedding_service.clone(),
        config.ai_use_streaming,
        config.ai_thinking_message.clone(),
        config.memory_recent_limit as usize,
        config.memory_relevant_top_k as usize,
        config.memory_semantic_min_score,
    ));

    Ok(BotComponents {
        repo,
        teloxide_bot,
        bot_username,
        sync_ai_handler,
        memory_store,
        recent_store,
        embedding_service,
    })
}

/// 初始化 Bot 的核心组件。
///
/// - 初始化消息存储（SQLite / 其他）。
/// - 初始化 Telegram Bot。
/// - 初始化 AI 查询通道与处理器。
/// - 初始化向量记忆存储。
///
/// 该函数不负责：
/// - 日志目录创建与 tracing 初始化（由 `run_bot` 调用方负责）。
/// - 启动 AI 查询处理器任务。
#[instrument(skip(config))]
pub async fn initialize_bot_components(config: &BotConfig) -> Result<BotComponents> {
    // 初始化内存存储
    let memory_store: Arc<dyn MemoryStore> = match config.memory_store_type.as_str() {
        "lance" => {
            let lance_path = config
                .memory_lance_path
                .clone()
                .unwrap_or_else(|| "./data/lance_db".to_string());
            // 与 embedding 服务维度一致：智谱 embedding-2 为 1024，OpenAI text-embedding-3-small 为 1536。
            // 若切换 provider，需删除已有 Lance 目录或确保表按新维度重建，否则 semantic_search 会报错。
            let embedding_dim = match config.embedding_provider.as_str() {
                "zhipuai" => 1024,
                _ => 1536,
            };
            let lance_config = LanceConfig {
                db_path: lance_path.clone(),
                embedding_dim,
                ..Default::default()
            };
            info!(
                db_path = %lance_path,
                embedding_dim = embedding_dim,
                "Using Lance vector store"
            );
            Arc::new(LanceVectorStore::with_config(lance_config).await.map_err(|e| {
                error!(error = %e, "Failed to initialize Lance store");
                anyhow::anyhow!("Failed to initialize Lance store: {}", e)
            })?)
        }
        "sqlite" => {
            info!(db_path = %config.memory_sqlite_path, "Using SQLite vector store");
            Arc::new(
                SQLiteVectorStore::new(&config.memory_sqlite_path)
                    .await
                    .map_err(|e| {
                        error!(error = %e, "Failed to initialize SQLite store");
                        anyhow::anyhow!("Failed to initialize SQLite store: {}", e)
                    })?,
            )
        }
        _ => {
            info!("Using in-memory vector store");
            Arc::new(InMemoryVectorStore::new())
        }
    };

    let recent_store: Option<Arc<dyn MemoryStore>> = if config.memory_recent_use_sqlite {
        info!(
            db_path = %config.memory_sqlite_path,
            "Using SQLite for recent messages (RecentMessagesStrategy / UserPreferencesStrategy)"
        );
        match SQLiteVectorStore::new(&config.memory_sqlite_path).await {
            Ok(s) => Some(Arc::new(s)),
            Err(e) => {
                error!(error = %e, "Failed to initialize SQLite store for recent messages");
                return Err(anyhow::anyhow!(
                    "MEMORY_RECENT_USE_SQLITE=true but failed to open SQLite: {}",
                    e
                ));
            }
        }
    } else {
        None
    };

    build_bot_components(config, memory_store, recent_store).await
}

/// 使用自定义 `MemoryStore` 初始化 Bot 组件。
///
/// - 不会根据 `config.memory_store_type` 再次选择实现，而是完全使用传入的 `memory_store`。
/// - 主要用于测试场景下注入 `MockMemoryStore` 等自定义实现。
#[instrument(skip(config, memory_store))]
pub async fn initialize_bot_components_with_store(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
) -> Result<BotComponents> {
    build_bot_components(config, memory_store, None).await
}

impl TelegramBot {
    /// 使用配置初始化一个新的 TelegramBot。
    pub async fn new(config: BotConfig) -> Result<Self> {
        let components = initialize_bot_components(&config).await?;

        // 初始化持久化中间件
        let persistence_middleware =
            Arc::new(PersistenceMiddleware::new(components.repo.as_ref().clone()));

        // 初始化记忆中间件（带 embedding 服务，写入时算向量以参与语义检索）
        let memory_middleware = Arc::new(MemoryMiddleware::with_store_and_embedding(
            components.memory_store.clone(),
            components.embedding_service.clone(),
            components.recent_store.clone(),
        ));

        // 构建处理器链（SyncAIHandler 在链内同步执行 AI，返回 Reply 供 memory_middleware 在 after() 中存 AI 回复）
        let handler_chain = HandlerChain::new()
            .add_middleware(persistence_middleware)
            .add_middleware(memory_middleware)
            .add_handler(components.sync_ai_handler.clone());

        Ok(Self {
            config,
            components,
            handler_chain,
        })
    }

    /// 使用自定义 MemoryStore 初始化 TelegramBot（主要用于测试）。
    pub async fn new_with_memory_store(
        config: BotConfig,
        memory_store: Arc<dyn MemoryStore>,
    ) -> Result<Self> {
        let components = initialize_bot_components_with_store(&config, memory_store).await?;

        // 初始化持久化中间件
        let persistence_middleware =
            Arc::new(PersistenceMiddleware::new(components.repo.as_ref().clone()));

        // 初始化记忆中间件（带 embedding 服务，写入时算向量以参与语义检索）
        let memory_middleware = Arc::new(MemoryMiddleware::with_store_and_embedding(
            components.memory_store.clone(),
            components.embedding_service.clone(),
            components.recent_store.clone(),
        ));

        // 构建处理器链
        let handler_chain = HandlerChain::new()
            .add_middleware(persistence_middleware)
            .add_middleware(memory_middleware)
            .add_handler(components.sync_ai_handler.clone());

        Ok(Self {
            config,
            components,
            handler_chain,
        })
    }

    /// 处理一条来自 Telegram 的消息（可在单元测试中直接调用）。
    pub async fn handle_message(&self, msg: &teloxide::types::Message) -> Result<()> {
        if let Some(text) = msg.text() {
            let wrapper = TelegramMessageWrapper(msg);
            let core_msg = wrapper.to_core();

            info!(
                user_id = core_msg.user.id,
                message_content = %text,
                "Received message"
            );

            if let Err(e) = self.handler_chain.handle(&core_msg).await {
                error!(error = %e, user_id = core_msg.user.id, "Handler chain failed");
            }
        }

        Ok(())
    }

    /// 使用 core 层消息直接驱动处理器链（仅用于集成测试，避免构造 teloxide Message）。
    ///
    /// 行为：与 handle_message 一致，但入参为 dbot_core::Message，便于测试中构造“回复机器人”等场景。
    #[doc(hidden)]
    pub async fn handle_core_message(&self, message: &CoreMessage) -> Result<()> {
        info!(
            user_id = message.user.id,
            message_content = %message.content,
            "Handling core message (test)"
        );
        if let Err(e) = self.handler_chain.handle(message).await {
            error!(error = %e, user_id = message.user.id, "Handler chain failed");
        }
        Ok(())
    }

}

/// 运行 Telegram Bot 的主入口
#[instrument(skip(config))]
pub async fn run_bot(config: BotConfig) -> Result<()> {
    // 初始化日志
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    init_tracing(&config.log_file)?;

    info!(
        database_url = %config.database_url,
        ai_model = %config.ai_model,
        ai_use_streaming = config.ai_use_streaming,
        memory_store_type = %config.memory_store_type,
        "Initializing bot"
    );

    // 使用 TelegramBot 结构封装后的初始化逻辑
    let bot = TelegramBot::new(config).await?;
    let handler_chain = bot.handler_chain.clone();
    let bot_username = bot.components.bot_username.clone();
    let teloxide_bot = bot.components.teloxide_bot.clone();

    info!("Bot started successfully");

    // 在启动 repl 前先设置 bot_username，否则首条 @ 消息到达时尚未设置会导致 SyncAIHandler 判定为“非 AI 查询”而不回复
    // 使用框架层 REPL：消息转 core::Message 后交给 HandlerChain；run_repl 内会 get_me 并写回 bot_username
    run_repl(teloxide_bot, handler_chain, bot_username).await?;

    Ok(())
}
