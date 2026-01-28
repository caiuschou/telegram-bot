use anyhow::Result;
use bot_runtime::{AIDetectionHandler, HandlerChain};
use dbot_core::{init_tracing, ToCoreMessage};
use middleware::{PersistenceMiddleware, MemoryMiddleware};
use memory::MemoryStore;
use memory_inmemory::InMemoryVectorStore;
use memory_sqlite::SQLiteVectorStore;
use openai_client::OpenAIClient;
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::adapters::TelegramMessageWrapper;
use super::config::BotConfig;

/// 运行 Telegram Bot 的主入口
#[instrument(skip(config))]
pub async fn run_bot(config: BotConfig) -> Result<()> {
    // 初始化日志
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    init_tracing(&config.log_file)?;

    info!(
        database_url = %config.database_url,
        ai_model = %config.ai_model,
        ai_use_streaming = config.ai_use_streaming,
        memory_store_type = %config.memory_store_type,
        "Initializing bot"
    );

    // 初始化存储
    let repo = Arc::new(
        MessageRepository::new(&config.database_url)
            .await
            .map_err(|e| {
                error!(error = %e, database_url = %config.database_url, "Failed to initialize message storage");
                e
            })
            .expect("Failed to initialize message storage"),
    );

    // 初始化 Telegram Bot
    let teloxide_bot = Bot::new(config.bot_token);

    // 存储 bot username
    let bot_username = Arc::new(tokio::sync::RwLock::new(None));

    // 创建 AI 查询通道
    let (query_sender, query_receiver) = tokio::sync::mpsc::unbounded_channel();

    // 初始化 OpenAI 客户端
    let openai_client = OpenAIClient::with_base_url(config.openai_api_key, config.openai_base_url);
    let ai_bot = TelegramBotAI::new("bot".to_string(), openai_client).with_model(config.ai_model);

    // 初始化内存存储
    let memory_store: Arc<dyn MemoryStore> = match config.memory_store_type.as_str() {
        "lance" => {
            // Note: Lance vector store is available when lance feature is enabled
            // For now, fall back to in-memory store
            info!("Lance feature not enabled, falling back to in-memory store");
            Arc::new(InMemoryVectorStore::new())
        }
        "sqlite" => {
            info!(db_path = %config.memory_sqlite_path, "Using SQLite vector store");
            Arc::new(SQLiteVectorStore::new(&config.memory_sqlite_path).await
                .map_err(|e| {
                    error!(error = %e, "Failed to initialize SQLite store");
                    e
                })?)
        }
        _ => {
            info!("Using in-memory vector store");
            Arc::new(InMemoryVectorStore::new())
        }
    };

    // 初始化 AI 查询处理器
    let mut ai_query_handler = bot_runtime::AIQueryHandler::new(
        ai_bot,
        teloxide_bot.clone(),
        repo.as_ref().clone(),
        memory_store.clone(),
        query_receiver,
        config.ai_use_streaming,
        config.ai_thinking_message,
    );

    // 启动 AI 查询处理器
    tokio::spawn(async move {
        ai_query_handler.run().await;
    });

    // 初始化 AI 检测处理器
    let ai_detection_handler = Arc::new(AIDetectionHandler::new(
        bot_username.clone(),
        Arc::new(query_sender),
    ));

    // 初始化持久化中间件
    let persistence_middleware = Arc::new(PersistenceMiddleware::new(repo.as_ref().clone()));

    // 初始化记忆中间件
    let memory_middleware = Arc::new(MemoryMiddleware::with_store(memory_store));

    // 构建处理器链
    let handler_chain = HandlerChain::new()
        .add_middleware(persistence_middleware)
        .add_middleware(memory_middleware)
        .add_handler(ai_detection_handler);

    info!("Bot started successfully");

    // 获取并设置 bot username
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

    // 启动 bot 监听
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
