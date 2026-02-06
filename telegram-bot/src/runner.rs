use anyhow::Result;
use std::sync::Arc;
use crate::core::{Bot, Handler, init_tracing, Message as CoreMessage, ToCoreMessage};
use crate::telegram::{run_repl, TelegramMessageWrapper};
use crate::chain::HandlerChain;
use crate::memory::MemoryStore;
use tracing::{error, info, instrument};

use super::components::{
    build_bot_components, build_handler_chain, create_memory_stores, BotComponents,
};
use super::config::{AppExtensions, BotConfig};

/// TelegramBot: config, components, and handler chain. Handler is injected from outside.
pub struct TelegramBot {
    pub config: BotConfig,
    pub components: BotComponents,
    pub handler_chain: HandlerChain,
}

impl TelegramBot {
    /// Creates a TelegramBot from config and handler. Handler is built externally using components.
    pub async fn new(config: BotConfig, handler: Arc<dyn Handler>) -> Result<Self> {
        let (memory_store, recent_store) = create_memory_stores(&config).await?;
        let components = build_bot_components(&config, memory_store, recent_store, None).await?;
        let handler_chain = build_handler_chain(&components, handler);
        Ok(Self {
            config,
            components,
            handler_chain,
        })
    }

    /// Creates TelegramBot with a custom MemoryStore (e.g. for tests).
    pub async fn new_with_memory_store(
        config: BotConfig,
        memory_store: Arc<dyn MemoryStore>,
        handler: Arc<dyn Handler>,
    ) -> Result<Self> {
        let components = build_bot_components(&config, memory_store, None, None).await?;
        let handler_chain = build_handler_chain(&components, handler);
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

    /// Drive handler chain with core Message (for integration tests).
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

/// Main entry: init logging, validate config, build components, create handler via factory, then run REPL.
/// The factory receives (config, BotComponents) and returns the handler (e.g. InlineLLMHandler built from llm-handlers).
#[instrument(skip(config, make_handler))]
pub async fn run_bot<F>(config: BotConfig, make_handler: F) -> Result<()>
where
    F: FnOnce(&BotConfig, BotComponents) -> Arc<dyn Handler>,
{
    let (memory_store, recent_store) = create_memory_stores(&config).await?;
    run_bot_with_memory_stores(config, memory_store, recent_store, make_handler).await
}

/// Runs the bot with pre-built memory stores.
/// Use this when the caller (e.g. telegram-llm-bot) injects custom stores such as Lance.
#[instrument(skip(config, memory_store, recent_store, make_handler))]
pub async fn run_bot_with_memory_stores<F>(
    config: BotConfig,
    memory_store: Arc<dyn MemoryStore>,
    recent_store: Option<Arc<dyn MemoryStore>>,
    make_handler: F,
) -> Result<()>
where
    F: FnOnce(&BotConfig, BotComponents) -> Arc<dyn Handler>,
{
    config.validate()?;
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    init_tracing(config.base().log_file.as_str())?;

    let mem_cfg = config
        .extensions()
        .memory_config()
        .expect("BaseAppExtensions always has memory");

    info!(
        database_url = %config.base().database_url,
        memory_store_type = %mem_cfg.store_type(),
        "Initializing bot"
    );

    let components = build_bot_components(&config, memory_store, recent_store, None).await?;
    let handler = make_handler(&config, components.clone());
    let handler_chain = build_handler_chain(&components, handler);
    let bot_username = components.bot_username.clone();
    let bot_user = components.bot_user.clone();
    let teloxide_bot = components.teloxide_bot.clone();

    info!("Bot started successfully");

    run_repl(teloxide_bot, handler_chain, bot_username, bot_user).await?;

    Ok(())
}

/// Builds components and handler chain without starting the REPL. Used by integration tests that inject a mock bot and drive the chain with fake messages.
///
/// When `handler_bot_override` is `Some`, it is passed to `build_bot_components` so that `make_handler` receives it in `components.handler_bot`.
#[instrument(skip(config, memory_store, recent_store, handler_bot_override, make_handler))]
pub async fn run_bot_with_memory_stores_build_only<F>(
    config: BotConfig,
    memory_store: Arc<dyn MemoryStore>,
    recent_store: Option<Arc<dyn MemoryStore>>,
    handler_bot_override: Option<Arc<dyn Bot>>,
    make_handler: F,
) -> Result<HandlerChain>
where
    F: FnOnce(&BotConfig, BotComponents) -> Arc<dyn Handler>,
{
    config.validate()?;
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    init_tracing(config.base().log_file.as_str())?;

    let mem_cfg = config
        .extensions()
        .memory_config()
        .expect("BaseAppExtensions always has memory");

    info!(
        database_url = %config.base().database_url,
        memory_store_type = %mem_cfg.store_type(),
        "Building bot (no REPL)"
    );

    let components = build_bot_components(&config, memory_store, recent_store, handler_bot_override).await?;
    let handler = make_handler(&config, components.clone());
    let handler_chain = build_handler_chain(&components, handler);

    Ok(handler_chain)
}
