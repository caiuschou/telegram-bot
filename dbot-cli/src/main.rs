use anyhow::Result;
use bot_runtime::{AIDetectionHandler, HandlerChain, MessageHandler};
use clap::Parser;
use dbot_core::{init_tracing, Chat, Message, MessageDirection, ToCoreMessage, ToCoreUser, User};
use openai_client::OpenAIClient;
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

#[derive(Parser)]
#[command(name = "dbot")]
#[command(about = "运行 Telegram Bot", long_about = None)]
#[command(version)]
struct Cli {
    /// Bot token（覆盖环境变量）
    #[arg(short, long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    run_bot(cli.token).await
}

#[instrument(skip(token))]
async fn run_bot(token: Option<String>) -> Result<()> {
    let log_file = "logs/telegram-bot.log";
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");

    init_tracing(log_file)?;

    let bot_token = token.unwrap_or_else(|| std::env::var("BOT_TOKEN").expect("BOT_TOKEN not set"));

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "file:./telegram_bot.db".to_string());

    let openai_api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let openai_base_url = std::env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let ai_model = std::env::var("AI_MODEL")
        .unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
    let ai_use_streaming = std::env::var("AI_USE_STREAMING")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(false);
    let ai_thinking_message = std::env::var("AI_THINKING_MESSAGE")
        .unwrap_or_else(|_| "正在思考...".to_string());

    info!(
        database_url = %database_url,
        ai_model = %ai_model,
        ai_use_streaming = ai_use_streaming,
        "Initializing bot"
    );

    let repo = Arc::new(
        MessageRepository::new(&database_url)
            .await
            .map_err(|e| {
                error!(error = %e, database_url = %database_url, "Failed to initialize message storage");
                e
            })
            .expect("Failed to initialize message storage"),
    );

    let teloxide_bot = Bot::new(bot_token);

    let bot_username = Arc::new(tokio::sync::RwLock::new(None));

    let (query_sender, query_receiver) = tokio::sync::mpsc::unbounded_channel();

    let openai_client = OpenAIClient::with_base_url(openai_api_key, openai_base_url);
    let ai_bot = TelegramBotAI::new("bot".to_string(), openai_client).with_model(ai_model);

    let mut ai_query_handler = bot_runtime::AIQueryHandler::new(
        ai_bot,
        teloxide_bot.clone(),
        repo.as_ref().clone(),
        query_receiver,
        ai_use_streaming,
        ai_thinking_message,
    );

    tokio::spawn(async move {
        ai_query_handler.run().await;
    });

    let ai_detection_handler = Arc::new(AIDetectionHandler::new(
        bot_username.clone(),
        Arc::new(query_sender),
    ));

    let message_handler = Arc::new(MessageHandler::new(repo.as_ref().clone()));

    let handler_chain = HandlerChain::new()
        .add_handler(ai_detection_handler)
        .add_handler(message_handler);

    info!("Bot started successfully");

    let bot = teloxide_bot.clone();
    let bot_username_ref = bot_username.clone();
    tokio::spawn(async move {
        if let Ok(me) = bot.get_me().await {
            if let Some(username) = &me.user.username {
                *bot_username_ref.write().await = Some(username.clone());
                info!(username = %username, "Bot username set");
            }
        }
    });

    teloxide::repl(
        teloxide_bot,
        move |_bot: Bot, msg: teloxide::types::Message| {
            let handler_chain = handler_chain.clone();

            async move {
                if let Some(text) = msg.text() {
                    let wrapper = TelegramMessageWrapper(&msg);
                    let core_msg = wrapper.to_core();

                    info!(
                        user_id = core_msg.user.id,
                        message_content = %text,
                        "Received message"
                    );

                    match handler_chain.handle(&core_msg).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!(error = %e, user_id = core_msg.user.id, "Handler chain failed");
                        }
                    }
                }
                Ok(())
            }
        },
    )
    .await;

    Ok(())
}

struct TelegramUserWrapper<'a>(&'a teloxide::types::User);

impl<'a> ToCoreUser for TelegramUserWrapper<'a> {
    fn to_core(&self) -> User {
        User {
            id: self.0.id.0 as i64,
            username: self.0.username.clone(),
            first_name: Some(self.0.first_name.clone()),
            last_name: self.0.last_name.clone(),
        }
    }
}

struct TelegramMessageWrapper<'a>(&'a teloxide::types::Message);

impl<'a> ToCoreMessage for TelegramMessageWrapper<'a> {
    fn to_core(&self) -> Message {
        Message {
            id: self.0.id.to_string(),
            user: self
                .0
                .from
                .as_ref()
                .map(|u| TelegramUserWrapper(u).to_core())
                .unwrap_or_else(|| User {
                    id: 0,
                    username: None,
                    first_name: None,
                    last_name: None,
                }),
            chat: Chat {
                id: self.0.chat.id.0,
                chat_type: format!("{:?}", self.0.chat.kind),
            },
            content: self.0.text().unwrap_or("").to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
        }
    }
}
