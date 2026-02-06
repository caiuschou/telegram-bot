//! REPL runner: converts teloxide messages to core::Message and passes them to HandlerChain. Calls teloxide REPL and optional get_me to populate bot_username and bot_user.
//!
//! ## Error handling
//!
//! Message handling runs inside a spawned task per message. If the handler chain fails,
//! the error is only logged (via `tracing::error`); no message is sent to the user.
//! This keeps the REPL responsive and avoids exposing internal errors to end users.

use crate::chain::HandlerChain;
use crate::core::{ToCoreMessage, ToCoreUser};
use anyhow::Result;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::adapters::{TelegramMessageWrapper, TelegramUserWrapper};

/// Starts the REPL with the given teloxide Bot, HandlerChain, and bot identity caches.
///
/// Calls `get_me()` before starting and writes the full bot [`User`](crate::core::User) into `bot_user`,
/// and the username into `bot_username` (for backward compatibility and @mention detection).
/// Each incoming message is converted to [`crate::core::Message`] and processed by
/// `chain.handle()` inside a spawned task (so the REPL returns immediately).
///
/// On handler chain failure, the error is only logged; the user is not notified.
#[instrument(skip(bot, handler_chain, bot_username, bot_user))]
pub async fn run_repl(
    bot: teloxide::Bot,
    handler_chain: HandlerChain,
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    bot_user: Arc<tokio::sync::RwLock<Option<crate::core::User>>>,
) -> Result<()> {
    if let Ok(me) = bot.get_me().await {
        let core_user = TelegramUserWrapper(&me.user).to_core();
        *bot_user.write().await = Some(core_user.clone());
        if let Some(username) = &me.user.username {
            *bot_username.write().await = Some(username.clone());
            info!(username = %username, bot_id = core_user.id, "Bot getMe: username and full user set before repl");
        } else {
            info!(bot_id = core_user.id, "Bot getMe: full user set before repl (no username)");
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
