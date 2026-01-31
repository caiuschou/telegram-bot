use anyhow::Result;
use dbot_core::{init_tracing, Message as CoreMessage, ToCoreMessage};
use dbot_telegram::{run_repl, TelegramMessageWrapper};
use handler_chain::HandlerChain;
use memory::MemoryStore;
use std::sync::Arc;
use tracing::{error, info, instrument};

use super::components::{
    build_handler_chain, initialize_bot_components, initialize_bot_components_with_store,
    BotComponents,
};
use super::config::BotConfig;

/// TelegramBot: config, components, and handler chain. Testable via handle_message / handle_core_message.
pub struct TelegramBot {
    pub config: BotConfig,
    pub components: BotComponents,
    pub handler_chain: HandlerChain,
}

impl TelegramBot {
    /// Creates a TelegramBot from config (repo, memory, LLM, middleware chain).
    pub async fn new(config: BotConfig) -> Result<Self> {
        let components = initialize_bot_components(&config).await?;
        let handler_chain = build_handler_chain(&components);
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
        let handler_chain = build_handler_chain(&components);
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

    /// Drive handler chain with core Message (for integration tests; avoids building teloxide Message).
    ///
    /// Same behavior as handle_message but takes dbot_core::Message for tests (e.g. reply-to-bot).
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

/// Main entry: init logging, validate config, create TelegramBot, then run REPL.
#[instrument(skip(config))]
pub async fn run_bot(config: BotConfig) -> Result<()> {
    config.validate()?;
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    init_tracing(&config.log_file)?;

    info!(
        database_url = %config.database_url,
        llm_model = %config.llm_model,
        llm_use_streaming = config.llm_use_streaming,
        memory_store_type = %config.memory_store_type,
        "Initializing bot"
    );

    // Use TelegramBot struct for init logic
    let bot = TelegramBot::new(config).await?;
    let handler_chain = bot.handler_chain.clone();
    let bot_username = bot.components.bot_username.clone();
    let teloxide_bot = bot.components.teloxide_bot.clone();

    info!("Bot started successfully");

    // run_repl calls get_me and sets bot_username before handling messages
    run_repl(teloxide_bot, handler_chain, bot_username).await?;

    Ok(())
}
