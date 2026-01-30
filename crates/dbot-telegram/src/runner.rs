//! REPL 运行：将 teloxide 消息转为 core::Message 后交给 HandlerChain 处理。
//! 与外部交互：调用 teloxide REPL、dbot_core::HandlerChain、可选 get_me 写回 bot_username。

use anyhow::Result;
use dbot_core::ToCoreMessage;
use handler_chain::HandlerChain;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::adapters::TelegramMessageWrapper;

/// 使用给定的 teloxide Bot、HandlerChain 和 bot_username 缓存启动 REPL。
/// 启动前会调用 get_me() 并写入 bot_username；每条消息转为 core::Message 后交给 chain.handle。
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
