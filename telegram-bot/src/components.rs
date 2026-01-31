//! Component factory: builds BotComponents from config. Handler is injected from outside (LLM impl).

use anyhow::Result;
use crate::chain::HandlerChain;
use crate::core::Handler;
use crate::embedding::{BigModelEmbedding, OpenAIEmbedding};
use crate::handlers::{MemoryHandler, PersistenceHandler};
use crate::memory::{InMemoryVectorStore, MemoryStore, SQLiteVectorStore};
use std::sync::Arc;
use crate::storage::MessageRepository;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::config::{AppExtensions, BotConfig};

/// Core dependencies for run_bot / TelegramBot; handler is injected from outside.
#[derive(Clone)]
pub struct BotComponents {
    pub repo: Arc<MessageRepository>,
    pub teloxide_bot: Bot,
    pub bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    pub memory_store: Arc<dyn MemoryStore>,
    pub recent_store: Option<Arc<dyn MemoryStore>>,
    pub embedding_service: Arc<dyn crate::embedding::EmbeddingService>,
}

/// Creates the primary memory store and optional recent store from config.
#[instrument(skip(config))]
pub async fn create_memory_stores(
    config: &BotConfig,
) -> Result<(Arc<dyn MemoryStore>, Option<Arc<dyn MemoryStore>>)> {
    let mem_cfg = config
        .extensions()
        .memory_config()
        .ok_or_else(|| anyhow::anyhow!("Memory config required"))?;

    let memory_store: Arc<dyn MemoryStore> = match mem_cfg.store_type() {
        "lance" => {
            return Err(anyhow::anyhow!(
                "MEMORY_STORE_TYPE=lance is not supported by telegram-bot directly. \
                 Use run_bot_with_memory_stores and pass a Lance store from telegram-llm-bot (build with --features lance)."
            ));
        }
        "sqlite" => {
            info!(db_path = %mem_cfg.sqlite_path(), "Using SQLite vector store");
            Arc::new(
                SQLiteVectorStore::new(mem_cfg.sqlite_path())
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

    let recent_store: Option<Arc<dyn MemoryStore>> = if mem_cfg.recent_use_sqlite() {
        info!(
            db_path = %mem_cfg.sqlite_path(),
            "Using SQLite for recent messages"
        );
        match SQLiteVectorStore::new(mem_cfg.sqlite_path()).await {
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

/// Builds BotComponents (repo, teloxide_bot, memory_store, embedding, etc.). Handler is built externally using these.
#[instrument(skip(config, memory_store, recent_store))]
pub async fn build_bot_components(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
    recent_store: Option<Arc<dyn MemoryStore>>,
) -> Result<BotComponents> {
    let emb_cfg = config
        .extensions()
        .embedding_config()
        .ok_or_else(|| anyhow::anyhow!("Embedding config required"))?;

    let repo = Arc::new(
        MessageRepository::new(config.base().database_url.as_str())
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    database_url = %config.base().database_url,
                    "Failed to initialize message storage"
                );
                anyhow::anyhow!("Failed to initialize message storage: {}", e)
            })?,
    );

    let teloxide_bot = {
        let bot = Bot::new(config.base().bot_token.clone());
        if let Some(ref url_str) = config.base().telegram_api_url {
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

    let embedding_service: Arc<dyn crate::embedding::EmbeddingService> = match emb_cfg.provider() {
        "zhipuai" => {
            if emb_cfg.bigmodel_api_key().is_empty() {
                error!("EMBEDDING_PROVIDER=zhipuai but BIGMODEL_API_KEY / ZHIPUAI_API_KEY not set");
                return Err(anyhow::anyhow!(
                    "BIGMODEL_API_KEY or ZHIPUAI_API_KEY required when EMBEDDING_PROVIDER=zhipuai"
                ));
            }
            info!("Using BigModel (Zhipu AI) embedding");
            Arc::new(BigModelEmbedding::with_api_key(
                emb_cfg.bigmodel_api_key().to_string(),
            ))
        }
        _ => {
            info!("Using OpenAI embedding");
            Arc::new(OpenAIEmbedding::with_api_key(emb_cfg.openai_api_key().to_string()))
        }
    };

    Ok(BotComponents {
        repo,
        teloxide_bot,
        bot_username,
        memory_store,
        recent_store,
        embedding_service,
    })
}

/// Builds the handler chain (persistence → memory → LLM handler). LLM handler is injected from outside.
pub fn build_handler_chain(
    components: &BotComponents,
    handler: Arc<dyn Handler>,
) -> HandlerChain {
    let persistence = Arc::new(PersistenceHandler::new(components.repo.as_ref().clone()));
    let memory = Arc::new(MemoryHandler::with_store_and_embedding(
        components.memory_store.clone(),
        components.embedding_service.clone(),
        components.recent_store.clone(),
    ));
    HandlerChain::new()
        .add_handler(persistence)
        .add_handler(memory)
        .add_handler(handler)
}
