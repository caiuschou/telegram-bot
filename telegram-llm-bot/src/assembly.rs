//! Assembly: builds the LLM handler and creates memory stores. Used by the [facade](crate::facade).

use anyhow::{anyhow, Result};
use std::env;
use std::sync::Arc;
use telegram_bot::TelegramBotAdapter;
use llm_client::{EnvLlmConfig, LlmClient, LlmConfig, OpenAILlmClient};
use tracing::{info, warn};
use telegram_bot::{
    AppExtensions, BotComponents, BotConfig,
    memory::MemoryStore,
};

use crate::handlers::SyncLLMHandler;

/// Builds the LLM handler from config and components.
pub(crate) fn build_llm_handler(
    config: &BotConfig,
    components: BotComponents,
) -> Result<Arc<dyn telegram_bot::Handler>> {
    let llm_cfg = EnvLlmConfig::from_env()?;
    let mem_cfg = config
        .extensions()
        .memory_config()
        .ok_or_else(|| anyhow::anyhow!("Memory config required"))?;

    let system_prompt = config
        .extensions()
        .llm_system_prompt()
        .map(String::from)
        .or_else(|| llm_cfg.system_prompt().map(String::from))
        .or_else(|| {
            env::var("LLM_SYSTEM_PROMPT")
                .or_else(|_| env::var("SYSTEM_PROMPT"))
                .ok()
                .filter(|s| !s.trim().is_empty())
        });

    if let Some(ref s) = system_prompt {
        let prefix: String = s.chars().take(50).collect();
        info!(len = s.len(), prefix = %prefix, "Using custom SYSTEM_PROMPT from env");
    } else {
        warn!("No SYSTEM_PROMPT/LLM_SYSTEM_PROMPT in env; using default (plain text, no Markdown)");
    }

    let llm_client: Arc<dyn LlmClient> = Arc::new(
        OpenAILlmClient::with_base_url(
            llm_cfg.api_key().to_string(),
            llm_cfg.base_url().to_string(),
        )
        .with_model(llm_cfg.model().to_string())
        .with_system_prompt_opt(system_prompt),
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
pub(crate) async fn create_memory_stores_for_llm(
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
