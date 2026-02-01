//! REPL runner: converts teloxide messages to core::Message and passes them to HandlerChain. Calls teloxide REPL and optional get_me to populate bot_username.

use crate::chain::HandlerChain;
use crate::core::ToCoreMessage;
use anyhow::Result;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::adapters::TelegramMessageWrapper;

/// Starts the REPL with the given teloxide Bot, HandlerChain, and bot_username cache.
/// Calls get_me() before starting and writes username into bot_username; each message is converted to core::Message and passed to chain.handle (spawned per message).
#[instrument(skip(bot, handler_chain, bot_username))]
pub async fn run_repl(
    bot: teloxide::Bot,
    handler_chain: HandlerChain,
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
) -> Result<()> {
    if let Ok(me) = bot.get_me().await {
        if let Some(username) = &me.user.username {
            *bot_username.write().await = Some(username.clone());
            info!(username = %username, "Bot username set before repl");
        }
    }

    let chain = handler_chain;
    teloxide::repl(
        bot,
        move |_bot: Bot, msg: teloxide::types::Message| {
            let chain = chain.clone();

            async move {
                let wrapper = TelegramMessageWrapper(&msg);
                let core_msg = wrapper.to_core();

                match msg.text() {
                    Some(text) => {
                        info!(
                            user_id = core_msg.user.id,
                            chat_id = core_msg.chat.id,
                            message_content = %text,
                            "Received message"
                        );
                    }
                    None => {
                        info!(
                            user_id = core_msg.user.id,
                            chat_id = core_msg.chat.id,
                            "Received non-text message"
                        );
                    }
                }

                // Run handler chain in a spawned task so REPL returns immediately
                let chain_for_task = chain.clone();
                tokio::spawn(async move {
                    info!(
                        user_id = core_msg.user.id,
                        chat_id = core_msg.chat.id,
                        message_id = %core_msg.id,
                        "step: processing message (handler chain started)"
                    );
                    if let Err(e) = chain_for_task.handle(&core_msg).await {
                        error!(error = %e, user_id = core_msg.user.id, "Handler chain failed");
                    }
                });

                Ok(())
            }
        },
    )
    .await;

    Ok(())
}
