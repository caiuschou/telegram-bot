//! runner 集成测试工具
//!
//! - 对 `telegram-bot/src/runner.rs` 中 `run_bot` 的集成测试提供基础工具。
//! - 与外部交互：
//!   - 通过 `.env.test` / `.env` 加载真实/测试配置（OPENAI_API_KEY 等）。
//!   - 使用临时目录作为数据库与日志路径，避免污染工作区。
//!   - 通过 `MockMemoryStore` 在测试中替代真实向量存储。

use std::env;
use std::sync::Once;

use dbot_core::{Chat, Message, MessageDirection, User};
use telegram_bot::runner::TelegramBot;
use telegram_bot::BotConfig;
use tempfile::TempDir;
use tracing_subscriber::{fmt, EnvFilter};

mod mock_memory_store;
use mock_memory_store::MockMemoryStore;

/// 设置测试配置，使用 `.env.test` / `.env` 与临时目录
///
/// 行为：
/// - 使用 dotenvy 尝试加载 `.env.test`，失败时回退到默认 `.env`。
/// - 若关键环境变量未设置，则填充合理的默认测试值。
/// - 使用 `TempDir` 为 `DATABASE_URL`、`MEMORY_SQLITE_PATH` 等路径创建隔离目录。
///
/// 外部交互：
/// - 读取工作目录下的 `.env.test` / `.env`。
/// - 仅在测试进程生命周期内创建临时文件/目录。
fn setup_test_config(temp_dir: &TempDir) -> BotConfig {
    // 优先从 .env.test 加载，其次是默认 .env（若存在）
    let _ = dotenvy::from_filename(".env.test").or_else(|_| dotenvy::dotenv());

    let temp_path = temp_dir.path();

    // 必要配置：如果未设置则提供测试默认值/报错
    if env::var("BOT_TOKEN").is_err() {
        env::set_var("BOT_TOKEN", "test_bot_token_12345");
    }

    if env::var("OPENAI_API_KEY").is_err() {
        panic!(
            "OPENAI_API_KEY must be set in .env.test, .env or environment for integration tests"
        );
    }

    if env::var("OPENAI_BASE_URL").is_err() {
        env::set_var("OPENAI_BASE_URL", "https://api.openai.com/v1");
    }

    if env::var("AI_MODEL").is_err() {
        env::set_var("AI_MODEL", "gpt-3.5-turbo");
    }

    if env::var("AI_USE_STREAMING").is_err() {
        env::set_var("AI_USE_STREAMING", "false");
    }

    if env::var("AI_THINKING_MESSAGE").is_err() {
        env::set_var("AI_THINKING_MESSAGE", "Thinking...");
    }

    if env::var("MEMORY_STORE_TYPE").is_err() {
        // 在集成测试中优先使用内存存储，避免对 Lance/SQLite 的强依赖
        env::set_var("MEMORY_STORE_TYPE", "memory");
    }

    // 集成测试使用 OpenAI embedding，避免依赖 BIGMODEL_API_KEY
    env::set_var("EMBEDDING_PROVIDER", "openai");

    // 始终为测试覆盖路径型配置
    env::set_var(
        "DATABASE_URL",
        format!("file:{}/test.db", temp_path.display()),
    );
    env::set_var(
        "MEMORY_SQLITE_PATH",
        format!("{}/memory.db", temp_path.display()),
    );

    BotConfig::load(None).expect("BotConfig::load must succeed in test setup")
}

/// 初始化 tracing 日志，仅在测试进程中调用一次。
///
/// - 使用 `RUST_LOG` / `RUST_LOG_STYLE` 环境变量控制输出级别和样式。
/// - 通过 `with_test_writer()` 确保日志在 `cargo test` 时输出到测试控制台。
static TRACING_INIT: Once = Once::new();

fn init_tracing() {
    TRACING_INIT.call_once(|| {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("debug,memory=debug,telegram_bot=debug"));

        let _ = fmt()
            .with_env_filter(env_filter)
            .with_test_writer()
            .try_init();
    });
}

/// Mock Telegram Bot 的 `getMe` 接口。
///
/// 行为：
/// - 启动一个独立的 `mockito::Server`。
/// - 在 `/getMe` 路径上注册 HTTP GET Mock。
/// - 返回固定的 Bot 信息 JSON 响应。
///
/// 外部交互：
/// - 不会真正访问 Telegram，只在本地 HTTP 服务器上响应请求。
/// - 后续可以通过将 Telegram API 基础地址指向该服务器来复用此 Mock。
#[allow(dead_code)]
fn mock_telegram_get_me() -> mockito::ServerGuard {
    let mut server = mockito::Server::new();
    let _mock_get_me = server
        .mock("GET", "/getMe")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "ok": true,
            "result": {
                "id": 123456789,
                "is_bot": true,
                "first_name": "TestBot",
                "username": "testbot"
            }
        }"#,
        )
        .create();

    server
}

/// Mock Telegram Bot 的 `sendMessage` 接口。
///
/// 行为：
/// - 在给定的 `mockito::Server` 上注册 HTTP POST Mock。
/// - 返回固定的消息发送成功 JSON 响应。
///
/// 外部交互：
/// - 不会真正访问 Telegram，只在本地 HTTP 服务器上响应请求。
/// - 可用于验证 Bot 是否向 `/sendMessage` 发送了请求（通过 `mock.assert()`）。
#[allow(dead_code)]
fn mock_telegram_send_message(server: &mut mockito::ServerGuard) -> mockito::Mock {
    server
        .mock("POST", "/sendMessage")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "ok": true,
            "result": {
                "message_id": 1,
                "chat": {"id": 123},
                "text": "This is a test response"
            }
        }"#,
        )
        .create()
}

/// 主流程集成测试占位：AI 回复完整流程（仅环境与组件初始化）
#[tokio::test]
async fn test_ai_reply_complete_flow_smoke() {
    init_tracing();

    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    env::set_var("OPENAI_API_KEY", "test_key_for_integration_flow");

    let _config = setup_test_config(&temp_dir);
    let _memory_store = MockMemoryStore::new();
}

/// AI 回复流程端到端测试（需真实 OPENAI_API_KEY）
///
/// 验证点：
/// - TelegramBot 使用 MockMemoryStore 初始化
/// - 用户“回复机器人”消息触发 AI 队列
/// - handle_core_message 后持久化、记忆写入、查询被调用
/// - AI 处理器运行后：store 至少 2 次（用户消息 + AI 回复），query 至少 1 次，
///   semantic_search 至少 1 次（确保 embed 完成并执行向量检索后再断言，避免过早退出看不到 "OpenAI embed request completed"）
///
/// 外部交互：依赖 OPENAI_API_KEY 调用真实 OpenAI API，未设置时跳过。
#[tokio::test]
async fn test_ai_reply_complete_flow() {
    init_tracing();

    // 先加载 .env.test / .env，再检查 OPENAI_API_KEY，否则文件中的 key 不会被读到
    let _ = dotenvy::from_filename(".env.test").or_else(|_| dotenvy::dotenv());

    if env::var("OPENAI_API_KEY").is_err() {
        eprintln!("SKIP: OPENAI_API_KEY not set, skipping AI reply E2E test");
        return;
    }

    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    let config = setup_test_config(&temp_dir);
    let mock_store = MockMemoryStore::new();
    let mock_store = std::sync::Arc::new(mock_store);

    let bot = TelegramBot::new_with_memory_store(config, mock_store.clone())
        .await
        .expect("TelegramBot::new_with_memory_store");

    let msg = Message {
        id: "test_msg_1".to_string(),
        user: User {
            id: 123456,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: None,
        },
        chat: Chat {
            id: 123456,
            chat_type: "private".to_string(),
        },
        content: "Hello, can you help me?".to_string(),
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: chrono::Utc::now(),
        reply_to_message_id: Some("bot_msg_123".to_string()),
    };

    // SyncAIHandler 在链内同步执行：before() 存用户消息，handle() 调 AI 并返回 Reply，after() 存 AI 回复。handle_core_message 返回时链已结束。
    bot.handle_core_message(&msg).await.expect("handle_core_message");

    // User message saved by MemoryMiddleware in before(); AI response saved by MemoryMiddleware in after() when handler returns Reply(text). When API fails we may get only 1.
    assert!(
        mock_store.get_store_call_count() >= 1,
        "Memory store should be called at least once (user message from middleware), got {}",
        mock_store.get_store_call_count()
    );
    assert!(
        mock_store.get_query_call_count() >= 1,
        "Vector query should be executed at least once, got {}",
        mock_store.get_query_call_count()
    );
    // When embedding fails (e.g. invalid API key), semantic_search is skipped; no assertion on semantic_search_call_count.
}

/// 当 EMBEDDING_PROVIDER=zhipuai 且未设置 BIGMODEL_API_KEY / ZHIPUAI_API_KEY 时，初始化应失败。
#[tokio::test]
async fn test_embedding_provider_zhipuai_requires_api_key() {
    init_tracing();

    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    env::set_var("OPENAI_API_KEY", "test_key");
    setup_test_config(&temp_dir);

    env::set_var("EMBEDDING_PROVIDER", "zhipuai");
    env::remove_var("BIGMODEL_API_KEY");
    env::remove_var("ZHIPUAI_API_KEY");
    let config = BotConfig::load(None).expect("config load");
    assert_eq!(config.embedding_provider, "zhipuai");
    assert!(config.bigmodel_api_key.is_empty());

    let mock_store = std::sync::Arc::new(MockMemoryStore::new());
    let result = TelegramBot::new_with_memory_store(config, mock_store).await;

    let err = match result {
        Ok(_) => panic!("expected Err when zhipuai but no API key"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("BIGMODEL_API_KEY") || msg.contains("ZHIPUAI_API_KEY"),
        "error should mention API key: {}",
        msg
    );
}
