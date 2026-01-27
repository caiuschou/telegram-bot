use chrono::Local;
use dbot_core::init_tracing;
use teloxide::prelude::*;
use tracing::{error, info, instrument, warn};

#[tokio::main]
async fn main() {
    let log_file = "logs/clock-bot.log";
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");

    init_tracing(log_file).expect("Failed to initialize logging");

    let bot = Bot::from_env();
    info!(start_time = %Local::now().format("%Y-%m-%d %H:%M:%S"), log_file = %log_file, "Clock Bot started");

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
        let chat_id = msg.chat.id.0;
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

        if let Some(text) = msg.text() {
            info!(user_id = user_id, chat_id = chat_id, command = %text, "Received command");
            
            if text == "/clock" {
                let now = Local::now().format("%Y-%m-%d %H:%M:%S");
                let message = format!("Current time: {}", now);
                
                match bot.send_message(msg.chat.id, message).await {
                    Ok(_) => info!(user_id = user_id, chat_id = chat_id, time = %now, "Sent time"),
                    Err(e) => error!(user_id = user_id, chat_id = chat_id, error = %e, "Failed to send time"),
                }
            } else {
                warn!(user_id = user_id, chat_id = chat_id, command = %text, "Unknown command");
            }
        }
        Ok(())
    })
    .await;
}
