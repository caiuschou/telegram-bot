use dbot_core::init_tracing;
use dotenvy::dotenv;
use teloxide::prelude::*;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    init_tracing("logs/telegram-bot-ai.log")?;

    let token = std::env::var("BOT_TOKEN").expect("BOT_TOKEN not set");
    let bot = Bot::new(token);

    let llm_client = ai_client::OpenAILlmClient::with_base_url(
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
        std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
    );

    let bot_username =
        std::env::var("TELEGRAM_BOT_USERNAME").unwrap_or_else(|_| "AI_BOT".to_string());

    let bot_ai = telegram_bot_ai::TelegramBotAI::new(bot_username, llm_client);

    info!("AI Bot started successfully");

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let bot_ai = bot_ai.clone();
        async move {
            if let Some(text) = msg.text() {
                info!(
                    user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0),
                    message_content = %text,
                    "Received message"
                );
            }
            if let Err(e) = bot_ai.handle_message_stream(bot, msg).await {
                error!(error = %e, "Error handling message");
            }
            Ok(())
        }
    })
    .await;

    Ok(())
}
