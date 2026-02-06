//! **Public API of this crate.** Only these 4 functions form the stable surface.
//!
//! Types such as [`InlineLLMHandler`], [`LLMDetectionHandler`], and [`LLMQuery`] are re-exported from the crate root for custom chains or queue-based usage.

use anyhow::Result;
use telegram_bot::{run_bot_with_memory_stores, run_bot_with_memory_stores_build_only, BotComponents, BotConfig};

use crate::assembly;

/// Builds the LLM handler from config and components. Re-exported from assembly.
pub fn build_llm_handler(
    config: &BotConfig,
    components: BotComponents,
) -> Result<std::sync::Arc<dyn telegram_bot::Handler>> {
    assembly::build_llm_handler(config, components)
}

/// Creates memory stores from config. Supports lance when built with `--features lance`. Re-exported from assembly.
pub async fn create_memory_stores_for_llm(
    config: &BotConfig,
) -> Result<(
    std::sync::Arc<dyn telegram_bot::memory::MemoryStore>,
    Option<std::sync::Arc<dyn telegram_bot::memory::MemoryStore>>,
)> {
    assembly::create_memory_stores_for_llm(config).await
}

/// Runs the bot with the built-in LLM handler. Load config with `telegram_bot::load_config` before calling.
pub async fn run_bot_with_llm(config: BotConfig) -> Result<()> {
    let (memory_store, recent_store) = create_memory_stores_for_llm(&config).await?;
    run_bot_with_memory_stores(config, memory_store, recent_store, |config, components| {
        build_llm_handler(config, components).expect("Failed to build LLM handler")
    })
    .await
}

/// Runs the bot with a custom handler. Uses same memory stores as `run_bot_with_llm`
/// (including lance when built with `--features lance`).
///
/// Load config with `telegram_bot::load_config` before calling.
pub async fn run_bot_with_custom_handler<F>(
    config: BotConfig,
    make_handler: F,
) -> Result<()>
where
    F: FnOnce(&BotConfig, BotComponents) -> std::sync::Arc<dyn telegram_bot::Handler>,
{
    let (memory_store, recent_store) = create_memory_stores_for_llm(&config).await?;
    telegram_bot::run_bot_with_memory_stores(config, memory_store, recent_store, make_handler)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Builds the same pipeline as [`run_bot_with_custom_handler`] (config, memory stores, components, make_handler) but does not start the REPL. Returns the handler chain for driving with fake messages in tests.
///
/// When `handler_bot_override` is `Some`, the handler will use that bot (e.g. a mock) instead of the real Telegram bot.
pub async fn run_bot_with_custom_handler_build_only<F>(
    config: BotConfig,
    handler_bot_override: Option<std::sync::Arc<dyn telegram_bot::Bot>>,
    make_handler: F,
) -> Result<telegram_bot::HandlerChain>
where
    F: FnOnce(&BotConfig, BotComponents) -> std::sync::Arc<dyn telegram_bot::Handler>,
{
    let (memory_store, recent_store) = create_memory_stores_for_llm(&config).await?;
    run_bot_with_memory_stores_build_only(
        config,
        memory_store,
        recent_store,
        handler_bot_override,
        make_handler,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{}", e))
}
