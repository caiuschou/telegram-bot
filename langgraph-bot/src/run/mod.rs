//! Run Telegram bot with ReAct agent.

use anyhow::Result;
use std::sync::Arc;
use langgraph::ConfigSection;
use telegram_bot::{load_config, TelegramBot};
use telegram_llm_bot::run_bot_with_custom_handler;
use crate::{AgentHandler, create_react_runner, ReactRunner};

/// Runs the Telegram bot with ReAct agent. Config from env; `token` overrides BOT_TOKEN if provided.
/// The bot handles reply-to-bot and @mention messages, streams responses with placeholder → chunk updates → final reply.
/// Short-term memory is disabled: each message is processed without conversation history.
/// Tools config summary is logged inside the handler factory so it appears after tracing is initialized.
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

        let bot_token = components.teloxide_bot.token().to_string();
        let bot = Arc::new(TelegramBot::new(bot_token));
        let bot_username = components.bot_username.clone();
        let handler = Arc::new(AgentHandler::new(
            runner.clone(),
            bot,
            bot_username,
            placeholder_message.clone(),
        ));
        handler
    })
    .await
}
