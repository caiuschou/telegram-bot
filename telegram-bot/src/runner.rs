use ai_handlers::{AIDetectionHandler, AIQuery, AIQueryHandler};
use anyhow::Result;
use dbot_core::{init_tracing, ToCoreMessage};
use handler_chain::HandlerChain;
use memory::MemoryStore;
use memory_inmemory::InMemoryVectorStore;
use memory_lance::LanceVectorStore;
use memory_sqlite::SQLiteVectorStore;
use middleware::{MemoryMiddleware, PersistenceMiddleware};
use openai_client::OpenAIClient;
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::prelude::*;
use tracing::{error, info, instrument};

use super::adapters::TelegramMessageWrapper;
use super::config::BotConfig;

/// Bot 组件集合，封装 run_bot / TelegramBot 所需的核心依赖，便于测试与复用。
pub struct BotComponents {
    /// 消息持久化仓库
    pub repo: Arc<MessageRepository>,
    /// Teloxide Bot 实例
    pub teloxide_bot: Bot,
    /// Bot 的用户名缓存
    pub bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    /// 发送到 AI 查询处理器的通道
    pub query_sender: tokio::sync::mpsc::UnboundedSender<AIQuery>,
    /// AI 查询处理器
    pub ai_query_handler: AIQueryHandler,
    /// 向量记忆存储
    pub memory_store: Arc<dyn MemoryStore>,
}

/// 可测试的 TelegramBot 结构，封装 Bot 配置与依赖。
pub struct TelegramBot {
    /// 运行所需的配置
    pub config: BotConfig,
    /// 封装后的 Bot 组件
    pub components: BotComponents,
    /// 处理消息的中间件链
    pub handler_chain: HandlerChain,
}

/// 使用给定的 `MemoryStore` 构建 Bot 组件。
///
/// 注意：此函数假设传入的 `memory_store` 已根据配置选择好具体实现，
/// 因此不会再根据 `config.memory_store_type` 做分支判断。
#[instrument(skip(config, memory_store))]
async fn build_bot_components(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
) -> Result<BotComponents> {
    // 初始化存储
    let repo = Arc::new(
        MessageRepository::new(&config.database_url)
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    database_url = %config.database_url,
                    "Failed to initialize message storage"
                );
                anyhow::anyhow!("Failed to initialize message storage: {}", e)
            })?,
    );

    // 初始化 Telegram Bot
    let teloxide_bot = Bot::new(config.bot_token.clone());

    // 存储 bot username
    let bot_username = Arc::new(tokio::sync::RwLock::new(None));

    // 创建 AI 查询通道
    let (query_sender, query_receiver) = tokio::sync::mpsc::unbounded_channel();

    // 初始化 OpenAI 客户端
    let openai_client = OpenAIClient::with_base_url(
        config.openai_api_key.clone(),
        config.openai_base_url.clone(),
    );
    let ai_bot =
        TelegramBotAI::new("bot".to_string(), openai_client).with_model(config.ai_model.clone());

    // 初始化 AI 查询处理器
    let ai_query_handler = AIQueryHandler::new(
        ai_bot,
        teloxide_bot.clone(),
        repo.as_ref().clone(),
        memory_store.clone(),
        query_receiver,
        config.ai_use_streaming,
        config.ai_thinking_message.clone(),
    );

    Ok(BotComponents {
        repo,
        teloxide_bot,
        bot_username,
        query_sender,
        ai_query_handler,
        memory_store,
    })
}

/// 初始化 Bot 的核心组件。
///
/// - 初始化消息存储（SQLite / 其他）。
/// - 初始化 Telegram Bot。
/// - 初始化 AI 查询通道与处理器。
/// - 初始化向量记忆存储。
///
/// 该函数不负责：
/// - 日志目录创建与 tracing 初始化（由 `run_bot` 调用方负责）。
/// - 启动 AI 查询处理器任务。
#[instrument(skip(config))]
pub async fn initialize_bot_components(config: &BotConfig) -> Result<BotComponents> {
    // 初始化内存存储
    let memory_store: Arc<dyn MemoryStore> = match config.memory_store_type.as_str() {
        "lance" => {
            let lance_path = config
                .memory_lance_path
                .clone()
                .unwrap_or_else(|| "./data/lance_db".to_string());
            info!(db_path = %lance_path, "Using Lance vector store");
            Arc::new(LanceVectorStore::new(&lance_path).await.map_err(|e| {
                error!(error = %e, "Failed to initialize Lance store");
                anyhow::anyhow!("Failed to initialize Lance store: {}", e)
            })?)
        }
        "sqlite" => {
            info!(db_path = %config.memory_sqlite_path, "Using SQLite vector store");
            Arc::new(
                SQLiteVectorStore::new(&config.memory_sqlite_path)
                    .await
                    .map_err(|e| {
                        error!(error = %e, "Failed to initialize SQLite store");
                        anyhow::anyhow!("Failed to initialize SQLite store: {}", e)
                    })?,
            )
        }
        _ => {
            info!("Using in-memory vector store");
            Arc::new(InMemoryVectorStore::new())
        }
    };

    build_bot_components(config, memory_store).await
}

/// 使用自定义 `MemoryStore` 初始化 Bot 组件。
///
/// - 不会根据 `config.memory_store_type` 再次选择实现，而是完全使用传入的 `memory_store`。
/// - 主要用于测试场景下注入 `MockMemoryStore` 等自定义实现。
#[instrument(skip(config, memory_store))]
pub async fn initialize_bot_components_with_store(
    config: &BotConfig,
    memory_store: Arc<dyn MemoryStore>,
) -> Result<BotComponents> {
    build_bot_components(config, memory_store).await
}

impl TelegramBot {
    /// 使用配置初始化一个新的 TelegramBot。
    pub async fn new(config: BotConfig) -> Result<Self> {
        let components = initialize_bot_components(&config).await?;

        // 初始化 AI 检测处理器
        let ai_detection_handler = Arc::new(AIDetectionHandler::new(
            components.bot_username.clone(),
            Arc::new(components.query_sender.clone()),
        ));

        // 初始化持久化中间件
        let persistence_middleware =
            Arc::new(PersistenceMiddleware::new(components.repo.as_ref().clone()));

        // 初始化记忆中间件
        let memory_middleware = Arc::new(MemoryMiddleware::with_store(
            components.memory_store.clone(),
        ));

        // 构建处理器链
        let handler_chain = HandlerChain::new()
            .add_middleware(persistence_middleware)
            .add_middleware(memory_middleware)
            .add_handler(ai_detection_handler);

        Ok(Self {
            config,
            components,
            handler_chain,
        })
    }

    /// 使用自定义 MemoryStore 初始化 TelegramBot（主要用于测试）。
    pub async fn new_with_memory_store(
        config: BotConfig,
        memory_store: Arc<dyn MemoryStore>,
    ) -> Result<Self> {
        let components = initialize_bot_components_with_store(&config, memory_store).await?;

        // 初始化 AI 检测处理器
        let ai_detection_handler = Arc::new(AIDetectionHandler::new(
            components.bot_username.clone(),
            Arc::new(components.query_sender.clone()),
        ));

        // 初始化持久化中间件
        let persistence_middleware =
            Arc::new(PersistenceMiddleware::new(components.repo.as_ref().clone()));

        // 初始化记忆中间件
        let memory_middleware = Arc::new(MemoryMiddleware::with_store(
            components.memory_store.clone(),
        ));

        // 构建处理器链
        let handler_chain = HandlerChain::new()
            .add_middleware(persistence_middleware)
            .add_middleware(memory_middleware)
            .add_handler(ai_detection_handler);

        Ok(Self {
            config,
            components,
            handler_chain,
        })
    }

    /// 处理一条来自 Telegram 的消息（可在单元测试中直接调用）。
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

    /// 启动 AI 查询处理器任务。
    pub fn start_ai_handler(self) {
        let mut handler = self.components.ai_query_handler;
        tokio::spawn(async move {
            handler.run().await;
        });
    }
}

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

    // 使用 TelegramBot 结构封装后的初始化逻辑
    let bot = TelegramBot::new(config).await?;
    let handler_chain = bot.handler_chain.clone();
    let bot_username = bot.components.bot_username.clone();
    let teloxide_bot = bot.components.teloxide_bot.clone();

    // 启动 AI 查询处理器（在后台任务中运行）
    bot.start_ai_handler();

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
                let wrapper = TelegramMessageWrapper(&msg);
                let core_msg = wrapper.to_core();

                if let Some(text) = msg.text() {
                    info!(
                        user_id = core_msg.user.id,
                        message_content = %text,
                        "Received message"
                    );
                }

                if let Err(e) = handler_chain.handle(&core_msg).await {
                    error!(error = %e, user_id = core_msg.user.id, "Handler chain failed");
                }

                Ok(())
            }
        },
    )
    .await;

    Ok(())
}
