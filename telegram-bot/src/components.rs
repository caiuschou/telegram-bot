//! Component factory: builds BotComponents from config. Isolates assembly logic from runner.

use anyhow::Result;
use bigmodel_embedding::BigModelEmbedding;
use dbot_telegram::TelegramBotAdapter;
use handler_chain::HandlerChain;
use llm_client::{LlmClient, OpenAILlmClient};
use llm_handlers::SyncLLMHandler;
use memory::MemoryStore;
use memory_inmemory::InMemoryVectorStore;
use memory_lance::{LanceConfig, LanceVectorStore};
use memory_sqlite::SQLiteVectorStore;
use middleware::{MemoryMiddleware, PersistenceMiddleware};
use openai_embedding::OpenAIEmbedding;
use std::sync::Arc;
use storage::MessageRepository;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::config::BotConfig;

/// Core dependencies for run_bot / TelegramBot; produced by the component factory.
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

/// Creates the primary memory store and optional recent store from config.
#[instrument(skip(config))]
pub async fn create_memory_stores(
    config: &BotConfig,
) -> Result<(Arc<dyn MemoryStore>, Option<Arc<dyn MemoryStore>>)> {
    let memory_store: Arc<dyn MemoryStore> = match config.memory_store_type.as_str() {
        "lance" => {
            let lance_path = config
                .memory_lance_path
                .clone()
                .unwrap_or_else(|| "./data/lance_db".to_string());
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

    Ok((memory_store, recent_store))
}

/// Builds BotComponents with the given memory_store and optional recent_store.
#[instrument(skip(config, memory_store, recent_store))]
pub async fn build_bot_components(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
    recent_store: Option<Arc<dyn MemoryStore>>,
) -> Result<BotComponents> {
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

    let llm_client: Arc<dyn LlmClient> = Arc::new(
        OpenAILlmClient::with_base_url(
            config.openai_api_key.clone(),
            config.openai_base_url.clone(),
        )
        .with_model(config.llm_model.clone())
        .with_system_prompt_opt(config.llm_system_prompt.clone()),
    );
    let bot_adapter: Arc<dyn dbot_core::Bot> =
        Arc::new(TelegramBotAdapter::new(teloxide_bot.clone()));

    let embedding_service: Arc<dyn embedding::EmbeddingService> =
        match config.embedding_provider.as_str() {
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

/// Initializes BotComponents from config (creates memory stores then builds components).
#[instrument(skip(config))]
pub async fn initialize_bot_components(config: &BotConfig) -> Result<BotComponents> {
    let (memory_store, recent_store) = create_memory_stores(config).await?;
    build_bot_components(config, memory_store, recent_store).await
}

/// Initializes BotComponents with a custom MemoryStore (e.g. for tests).
#[instrument(skip(config, memory_store))]
pub async fn initialize_bot_components_with_store(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
) -> Result<BotComponents> {
    build_bot_components(config, memory_store, None).await
}

/// Builds the handler chain (persistence → memory → sync LLM handler).
pub fn build_handler_chain(components: &BotComponents) -> HandlerChain {
    let persistence_middleware =
        Arc::new(PersistenceMiddleware::new(components.repo.as_ref().clone()));
    let memory_middleware = Arc::new(MemoryMiddleware::with_store_and_embedding(
        components.memory_store.clone(),
        components.embedding_service.clone(),
        components.recent_store.clone(),
    ));
    HandlerChain::new()
        .add_middleware(persistence_middleware)
        .add_middleware(memory_middleware)
        .add_handler(components.sync_llm_handler.clone())
}
