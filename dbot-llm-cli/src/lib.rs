//! # dbot_llm
//!
//! LLM integration for Telegram bot: builds SyncLLMHandler and runs via telegram-bot.
//! Integrated into dbot-llm-cli crate.

use anyhow::Result;
use dbot_telegram::TelegramBotAdapter;
use llm_client::{EnvLlmConfig, LlmClient, LlmConfig, OpenAILlmClient};
use llm_handlers::SyncLLMHandler;
use std::sync::Arc;
use telegram_bot::{run_bot, AppExtensions, BotComponents, BotConfig};

/// Builds the LLM handler from config and components.
pub fn build_llm_handler(
    config: &BotConfig,
    components: BotComponents,
) -> Result<Arc<dyn dbot_core::Handler>> {
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

    let bot_adapter: Arc<dyn dbot_core::Bot> =
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

/// Runs the bot with LLM handler. Load config with `dbot_cli::load_config` before calling.
pub async fn run_bot_with_llm(config: BotConfig) -> Result<()> {
    run_bot(config, |config, components| {
        build_llm_handler(config, components).expect("Failed to build LLM handler")
    })
    .await
}
