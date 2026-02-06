//! Run Telegram bot with ReAct agent.

use anyhow::Result;
use std::sync::Arc;
use langgraph::ConfigSection;
use telegram_bot::{load_config, Bot, TelegramBot};
use telegram_bot::embedding::EmbeddingService;
use telegram_bot::memory::MemoryStore;
use telegram_llm_bot::run_bot_with_custom_handler;
use crate::telegram::{AgentHandler, EnsureLongTermMemoryHandler, EnsureThenAgentHandler, RunnerResolver};
use crate::{create_react_runner, ReactRunner};

/// Builds the handler chain used by `run_telegram`: EnsureThenAgentHandler → EnsureLongTermMemoryHandler → AgentHandler.
/// When `memory_store` and `embedding_service` are `None`, `RunnerResolver` is created with `None, None` (e.g. for tests).
#[allow(clippy::too_many_arguments)]
pub fn build_run_telegram_handler(
    runner: Arc<ReactRunner>,
    bot: Arc<dyn Bot>,
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    bot_user: Arc<tokio::sync::RwLock<Option<telegram_bot::User>>>,
    placeholder_message: String,
    memory_store: Option<Arc<dyn MemoryStore>>,
    embedding_service: Option<Arc<dyn EmbeddingService>>,
) -> Arc<dyn telegram_bot::Handler> {
    let runner_resolver = Arc::new(RunnerResolver::new(
        runner.clone(),
        memory_store,
        embedding_service,
    ));
    let ensure_handler = EnsureLongTermMemoryHandler::new(runner, bot_user.clone());
    let agent_handler = AgentHandler::new(
        runner_resolver,
        bot,
        bot_username,
        bot_user,
        placeholder_message,
    );
    Arc::new(EnsureThenAgentHandler::new(ensure_handler, agent_handler))
}

/// Runs the Telegram bot with ReAct agent. Config from env; `token` overrides BOT_TOKEN if provided.
/// The bot handles reply-to-bot and @mention messages, streams responses with placeholder → chunk updates → final reply.
/// Short-term memory is disabled: each message is processed without conversation history.
/// Tools config summary is logged inside the handler factory so it appears after tracing is initialized.
///
/// Logging is initialized by the telegram-bot runner: logs go to both stdout and the file given by
/// the `LOG_FILE` env var (default `logs/telegram-bot.log`). Set `LOG_FILE=logs/langgraph-bot.log`
/// in `.env` to use a dedicated log file for this bot.
pub async fn run_telegram(token: Option<String>) -> Result<()> {
    let config = load_config(token)?;

    let (runner, tool_summary, memory_summary) = create_react_runner().await?;
    let runner: Arc<ReactRunner> = Arc::new(runner);
    let placeholder_message = "正在思考…".to_string();

    run_bot_with_custom_handler(config, move |_config, components| {
        let memory_line: String = memory_summary
            .entries()
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" ");
        tracing::info!("[{}] {}", memory_summary.section_name(), memory_line);
        let tools_line: String = tool_summary
            .entries()
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" ");
        tracing::info!("[{}] {}", tool_summary.section_name(), tools_line);

        let bot = components
            .handler_bot
            .clone()
            .unwrap_or_else(|| Arc::new(TelegramBot::new(components.teloxide_bot.token().to_string())));
        build_run_telegram_handler(
            runner.clone(),
            bot,
            components.bot_username.clone(),
            components.bot_user.clone(),
            placeholder_message.clone(),
            Some(components.memory_store.clone()),
            Some(components.embedding_service.clone()),
        )
    })
    .await
}
