use llm_handlers::SyncLLMHandler;
use anyhow::Result;
use dbot_core::{init_tracing, Message as CoreMessage, ToCoreMessage};
use dbot_telegram::{run_repl, TelegramMessageWrapper};
use handler_chain::HandlerChain;
use memory::MemoryStore;
use memory_inmemory::InMemoryVectorStore;
use memory_lance::{LanceConfig, LanceVectorStore};
use memory_sqlite::SQLiteVectorStore;
use middleware::{MemoryMiddleware, PersistenceMiddleware};
use llm_client::OpenAILlmClient;
use bigmodel_embedding::BigModelEmbedding;
use dbot_telegram::TelegramBotAdapter;
use openai_embedding::OpenAIEmbedding;
use std::sync::Arc;
use storage::MessageRepository;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::config::BotConfig;

/// Core dependencies for run_bot / TelegramBot; shared for tests and reuse.
pub struct BotComponents {
    pub repo: Arc<MessageRepository>,
    pub teloxide_bot: Bot,
    pub bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    pub sync_llm_handler: Arc<SyncLLMHandler>,
    pub memory_store: Arc<dyn MemoryStore>,
    /// When set, RecentMessagesStrategy / UserPreferencesStrategy read from here; middleware writes to both main store and recent_store.
    pub recent_store: Option<Arc<dyn MemoryStore>>,
    pub embedding_service: Arc<dyn embedding::EmbeddingService>,
}

/// TelegramBot: config, components, and handler chain. Testable via handle_message / handle_core_message.
pub struct TelegramBot {
    pub config: BotConfig,
    pub components: BotComponents,
    pub handler_chain: HandlerChain,
}

/// Builds BotComponents with the given memory_store and optional recent_store. Does not branch on config.memory_store_type.
#[instrument(skip(config, memory_store, recent_store))]
async fn build_bot_components(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
    recent_store: Option<Arc<dyn MemoryStore>>,
) -> Result<BotComponents> {
    // Message persistence
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

    // Telegram Bot; if TELEGRAM_API_URL / TELOXIDE_API_URL is set (e.g. mock server), use it
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

    let bot_username = Arc::new(tokio::sync::RwLock::new(None));

    // LLM client and Bot adapter
    let llm_client = Arc::new(
        OpenAILlmClient::with_base_url(
            config.openai_api_key.clone(),
            config.openai_base_url.clone(),
        )
        .with_model(config.llm_model.clone())
        .with_system_prompt_opt(config.llm_system_prompt.clone()),
    );
    let bot_adapter: Arc<dyn dbot_core::Bot> =
        Arc::new(TelegramBotAdapter::new(teloxide_bot.clone()));

    // Embedding service for RAG semantic search
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

    // Sync LLM handler (returns Reply so MemoryMiddleware can save in after())
    let sync_llm_handler = Arc::new(SyncLLMHandler::new(
        bot_username.clone(),
        llm_client,
        bot_adapter,
        repo.as_ref().clone(),
        memory_store.clone(),
        recent_store.clone(),
        embedding_service.clone(),
        config.llm_use_streaming,
        config.llm_thinking_message.clone(),
        config.memory_recent_limit as usize,
        config.memory_relevant_top_k as usize,
        config.memory_semantic_min_score,
        config.telegram_edit_interval_secs,
    ));

    Ok(BotComponents {
        repo,
        teloxide_bot,
        bot_username,
        sync_llm_handler,
        memory_store,
        recent_store,
        embedding_service,
    })
}

/// Initializes core bot components: message repo, Telegram Bot, LLM handler, memory store. Does not create log dir or init tracing (caller does); does not start REPL.
#[instrument(skip(config))]
pub async fn initialize_bot_components(config: &BotConfig) -> Result<BotComponents> {
    let memory_store: Arc<dyn MemoryStore> = match config.memory_store_type.as_str() {
        "lance" => {
            let lance_path = config
                .memory_lance_path
                .clone()
                .unwrap_or_else(|| "./data/lance_db".to_string());
            // Embedding dim must match provider (Zhipu 1024, OpenAI 1536). If switching provider, recreate Lance DB or table.
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

/// Initializes BotComponents with a custom MemoryStore (e.g. MockMemoryStore for tests). Ignores config.memory_store_type.
#[instrument(skip(config, memory_store))]
pub async fn initialize_bot_components_with_store(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
) -> Result<BotComponents> {
    build_bot_components(config, memory_store, None).await
}

impl TelegramBot {
    /// Creates a TelegramBot from config (repo, memory, LLM, middleware chain).
    pub async fn new(config: BotConfig) -> Result<Self> {
        let components = initialize_bot_components(&config).await?;

        let persistence_middleware =
            Arc::new(PersistenceMiddleware::new(components.repo.as_ref().clone()));

        // 初始化记忆中间件（带 embedding 服务，写入时算向量以参与语义检索）
        let memory_middleware = Arc::new(MemoryMiddleware::with_store_and_embedding(
            components.memory_store.clone(),
            components.embedding_service.clone(),
            components.recent_store.clone(),
        ));

        let handler_chain = HandlerChain::new()
            .add_middleware(persistence_middleware)
            .add_middleware(memory_middleware)
            .add_handler(components.sync_llm_handler.clone());

        Ok(Self {
            config,
            components,
            handler_chain,
        })
    }

    /// Creates TelegramBot with a custom MemoryStore (for tests).
    pub async fn new_with_memory_store(
        config: BotConfig,
        memory_store: Arc<dyn MemoryStore>,
    ) -> Result<Self> {
        let components = initialize_bot_components_with_store(&config, memory_store).await?;

        let persistence_middleware =
            Arc::new(PersistenceMiddleware::new(components.repo.as_ref().clone()));

        let memory_middleware = Arc::new(MemoryMiddleware::with_store_and_embedding(
            components.memory_store.clone(),
            components.embedding_service.clone(),
            components.recent_store.clone(),
        ));

        // 构建处理器链
        let handler_chain = HandlerChain::new()
            .add_middleware(persistence_middleware)
            .add_middleware(memory_middleware)
            .add_handler(components.sync_llm_handler.clone());

        Ok(Self {
            config,
            components,
            handler_chain,
        })
    }

    /// Handles one Telegram message (callable from tests).
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

/// Main entry: init logging, create TelegramBot, then run REPL.
#[instrument(skip(config))]
pub async fn run_bot(config: BotConfig) -> Result<()> {
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    init_tracing(&config.log_file)?;

    info!(
        database_url = %config.database_url,
        llm_model = %config.llm_model,
        llm_use_streaming = config.llm_use_streaming,
        memory_store_type = %config.memory_store_type,
        "Initializing bot"
    );

    // 使用 TelegramBot 结构封装后的初始化逻辑
    let bot = TelegramBot::new(config).await?;
    let handler_chain = bot.handler_chain.clone();
    let bot_username = bot.components.bot_username.clone();
    let teloxide_bot = bot.components.teloxide_bot.clone();

    info!("Bot started successfully");

    // run_repl calls get_me and sets bot_username before handling messages
    run_repl(teloxide_bot, handler_chain, bot_username).await?;

    Ok(())
}
