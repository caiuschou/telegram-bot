//! # telegram_llm_bot
//!
//! LLM integration for Telegram bot: builds SyncLLMHandler and runs via telegram-bot.
//! Integrated into telegram-llm-bot crate.

pub mod llm_handlers;

use anyhow::{anyhow, Result};
use telegram_bot::TelegramBotAdapter;
use llm_client::{EnvLlmConfig, LlmClient, LlmConfig, OpenAILlmClient};
use crate::llm_handlers::SyncLLMHandler;
use std::sync::Arc;
use telegram_bot::{
    run_bot_with_memory_stores, AppExtensions, BotComponents, BotConfig,
    memory::MemoryStore,
};

/// Builds the LLM handler from config and components.
pub fn build_llm_handler(
    config: &BotConfig,
    components: BotComponents,
) -> Result<Arc<dyn telegram_bot::Handler>> {
    let llm_cfg = EnvLlmConfig::from_env()?;
    let mem_cfg = config
        .extensions()
        .memory_config()
        .ok_or_else(|| anyhow::anyhow!("Memory config required"))?;

    let llm_client: Arc<dyn LlmClient> = Arc::new(
        OpenAILlmClient::with_base_url(
            llm_cfg.api_key().to_string(),
            llm_cfg.base_url().to_string(),
        )
        .with_model(llm_cfg.model().to_string())
        .with_system_prompt_opt(llm_cfg.system_prompt().map(String::from)),
    );

    let bot_adapter: Arc<dyn telegram_bot::Bot> =
        Arc::new(TelegramBotAdapter::new(components.teloxide_bot.clone()));

    let handler = Arc::new(SyncLLMHandler::new(
        components.bot_username.clone(),
        llm_client,
        bot_adapter,
        components.repo.as_ref().clone(),
        components.memory_store.clone(),
        components.recent_store.clone(),
        components.embedding_service.clone(),
        llm_cfg.use_streaming(),
        llm_cfg.thinking_message().to_string(),
        mem_cfg.recent_limit() as usize,
        mem_cfg.relevant_top_k() as usize,
        mem_cfg.semantic_min_score(),
        config.base().telegram_edit_interval_secs,
    ));

    Ok(handler)
}

/// Creates memory stores from config. Supports lance when built with `--features lance`.
pub async fn create_memory_stores_for_llm(
    config: &BotConfig,
) -> Result<(Arc<dyn MemoryStore>, Option<Arc<dyn MemoryStore>>)> {
    let mem_cfg = config
        .extensions()
        .memory_config()
        .ok_or_else(|| anyhow!("Memory config required"))?;

    match mem_cfg.store_type() {
        "lance" => {
            #[cfg(feature = "lance")]
            {
                let lance_path = mem_cfg
                    .lance_path()
                    .unwrap_or("./data/lance_db")
                    .to_string();
                let emb_cfg = config.extensions().embedding_config();
                let embedding_dim = if emb_cfg.map_or(false, |e: &dyn telegram_bot::embedding::EmbeddingConfig| {
                    e.provider().eq_ignore_ascii_case("zhipuai")
                }) {
                    1024
                } else {
                    1536
                };
                let lance_config = memory_lance::LanceConfig {
                    db_path: lance_path.clone(),
                    embedding_dim,
                    ..Default::default()
                };
                let store = memory_lance::LanceVectorStore::with_config(lance_config).await?;
                let memory_store = Arc::new(store) as Arc<dyn MemoryStore>;

                let recent_store = if mem_cfg.recent_use_sqlite() {
                    Some(
                        Arc::new(
                            telegram_bot::memory::SQLiteVectorStore::new(mem_cfg.sqlite_path())
                                .await?,
                        ) as Arc<dyn MemoryStore>,
                    )
                } else {
                    None
                };

                Ok((memory_store, recent_store))
            }
            #[cfg(not(feature = "lance"))]
            Err(anyhow!(
                "MEMORY_STORE_TYPE=lance but telegram-llm-bot was built without the 'lance' feature. \
                 Build with --features lance."
            ))
        }
        _ => telegram_bot::create_memory_stores(config).await,
    }
}

/// Runs the bot with LLM handler. Load config with `telegram_bot::load_config` before calling.
pub async fn run_bot_with_llm(config: BotConfig) -> Result<()> {
    let (memory_store, recent_store) = create_memory_stores_for_llm(&config).await?;
    run_bot_with_memory_stores(config, memory_store, recent_store, |config, components| {
        build_llm_handler(config, components).expect("Failed to build LLM handler")
    })
    .await
}
