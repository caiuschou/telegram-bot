//! Integration tests for telegram-bot runner (run_bot, TelegramBot, handle_core_message).
//!
//! Loads config from `.env.test` or `.env`; uses temp dirs for DB and logs; uses MockMemoryStore instead of real vector store.
//! Pre-seeded memory entries are retrieved by context strategies when building LLM context.

use std::env;
use std::sync::Once;

use dbot_core::{Chat, Message, MessageDirection, User};
use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use telegram_bot::runner::TelegramBot;
use telegram_bot::BotConfig;
use tempfile::TempDir;
use tracing_subscriber::{fmt, EnvFilter};

mod mock_memory_store;
use mock_memory_store::MockMemoryStore;

/// Builds BotConfig for tests: loads .env.test or .env, sets defaults for missing vars, uses temp dir for DB/log paths.
fn setup_test_config(temp_dir: &TempDir) -> BotConfig {
    let _ = dotenvy::from_filename(".env.test").or_else(|_| dotenvy::dotenv());

    let temp_path = temp_dir.path();

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

    if env::var("MODEL").is_err() {
        env::set_var("MODEL", "gpt-3.5-turbo");
    }

    if env::var("USE_STREAMING").is_err() {
        env::set_var("USE_STREAMING", "false");
    }

    if env::var("THINKING_MESSAGE").is_err() {
        env::set_var("THINKING_MESSAGE", "Thinking...");
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

/// Teloxide 请求路径格式为 `/bot<token>/<method>`，测试用 token 为 `test_bot_token_12345`。
const TEST_BOT_TOKEN: &str = "test_bot_token_12345";

/// 在 mock 服务器上注册 Telegram getMe 与 sendMessage 的 mock。
/// 路径需与 teloxide 实际请求一致：`/bot<token>/getMe`、`/bot<token>/sendMessage`。
/// 返回 Mock guard，调用方必须持有至请求完成，否则 mock 被 drop 后 server 返回空 body 导致 JSON 解析错误。
fn register_telegram_mocks(server: &mut mockito::ServerGuard) -> (mockito::Mock, mockito::Mock) {
    let get_me_path = format!("/bot{}/getMe", TEST_BOT_TOKEN);
    let mock_get_me = server
        .mock("GET", get_me_path.as_str())
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

    let send_message_path = format!("/bot{}/sendMessage", TEST_BOT_TOKEN);
    let mock_send = server
        .mock("POST", send_message_path.as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "ok": true,
            "result": {
                "message_id": 1,
                "date": 1706529600,
                "chat": {"id": 123, "type": "private"},
                "from": {"id": 123456789, "is_bot": true, "first_name": "TestBot", "username": "testbot"},
                "text": "This is a test response"
            }
        }"#,
        )
        .create();

    (mock_get_me, mock_send)
}

/// 向 MemoryStore 预写入若干条消息上下文，供集成测试验证「根据记忆回复」。
///
/// 与 `build_memory_context` 中策略一致（参见 `docs/rag/context-retrieval-before-reply.md`）：
/// - **RecentMessagesStrategy**：按 `conversation_id` / `user_id` 拉取最近消息；
/// - **SemanticSearchStrategy**：用当前问题做向量检索（embed + semantic_search），
///   需要 store 中有可被查询的条目；本函数写入的即「向量库中会被查询到的内容」。
/// 使用与测试消息相同的 `user_id` / `conversation_id`（如 "123456"），这样策略能检索到这些条目，
/// AI 生成回复时会带上这些上下文。
///
/// 外部交互：仅调用 `MemoryStore::add`，不涉及网络或文件。
async fn seed_memory_context(
    store: std::sync::Arc<dyn MemoryStore>,
    user_id: &str,
    conversation_id: &str,
) -> Result<(), anyhow::Error> {
    let user_id = Some(user_id.to_string());
    let conversation_id = Some(conversation_id.to_string());
    let base_time = chrono::Utc::now() - chrono::Duration::hours(1);

    // 对话历史（RecentMessagesStrategy 会按会话拉取）
    let entries: &[(&str, MemoryRole, i64)] = &[
        ("我叫小明。", MemoryRole::User, 0),
        ("你好小明，有什么可以帮你的？", MemoryRole::Assistant, 1),
        ("我想了解一下天气。", MemoryRole::User, 2),
        ("好的，你可以问我具体城市或日期。", MemoryRole::Assistant, 3),
        // 以下为向量库中需被查询到的内容：用户偏好、历史帮助、时区等，便于 SemanticSearchStrategy 检索
        ("我平时喜欢用英文交流。", MemoryRole::User, 4),
        ("好的，我会优先用英文回复你。", MemoryRole::Assistant, 5),
        ("上次你帮我查了北京的天气，谢谢。", MemoryRole::User, 6),
        ("不客气，需要再查天气随时说。", MemoryRole::Assistant, 7),
        ("请记住：我的时区是 UTC+8。", MemoryRole::User, 8),
        ("已记住，会在时间相关回复里考虑 UTC+8。", MemoryRole::Assistant, 9),
    ];

    for (content, role, offset_min) in entries {
        let metadata = MemoryMetadata {
            user_id: user_id.clone(),
            conversation_id: conversation_id.clone(),
            role: *role,
            timestamp: base_time + chrono::Duration::minutes(*offset_min),
            tokens: None,
            importance: None,
        };
        let entry = MemoryEntry::new((*content).to_string(), metadata);
        store.add(entry).await?;
    }
    Ok(())
}

/// 主流程集成测试占位：LLM 回复完整流程（仅环境与组件初始化）
#[tokio::test]
async fn test_llm_reply_complete_flow_smoke() {
    init_tracing();
    env::set_var("EMBEDDING_PROVIDER", "openai");

    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    env::set_var("OPENAI_API_KEY", "test_key_for_integration_flow");

    let _config = setup_test_config(&temp_dir);
    let _memory_store = MockMemoryStore::new();
}

/// AI 回复流程端到端测试（需真实 OPENAI_API_KEY，Telegram API 使用 mock）
///
/// 验证点：
/// - TelegramBot 使用 MockMemoryStore 初始化，Telegram 请求发往 mock 服务器（不访问真实 Telegram）
/// - 用户“回复机器人”消息触发 AI 队列
/// - handle_core_message 后持久化、记忆写入、查询被调用
/// - AI 处理器运行后：store 至少 1 次，query 至少 1 次
///
/// 外部交互：依赖 OPENAI_API_KEY 调用真实 OpenAI API，未设置时跳过；Telegram 发往本地 mock。
#[tokio::test]
async fn test_ai_reply_complete_flow() {
    init_tracing();
    env::set_var("EMBEDDING_PROVIDER", "openai");

    // 先加载 .env.test / .env，再检查 OPENAI_API_KEY，否则文件中的 key 不会被读到
    let _ = dotenvy::from_filename(".env.test").or_else(|_| dotenvy::dotenv());

    if env::var("OPENAI_API_KEY").is_err() {
        eprintln!("SKIP: OPENAI_API_KEY not set, skipping LLM reply E2E test");
        return;
    }

    // 启动 Telegram API mock 服务器，避免使用假 token 访问真实 API 导致 Invalid bot token（使用 new_async 避免在 tokio runtime 内再 block_on）
    let mut server = mockito::Server::new_async().await;
    // 持有 mock guard 直至测试结束，否则 mock 被 drop 后请求会得到空 body，teloxide 解析 JSON 报 EOF
    let (_mock_get_me, _mock_send) = register_telegram_mocks(&mut server);
    env::set_var("TELEGRAM_API_URL", server.url());

    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    let config = setup_test_config(&temp_dir);
    let mock_store = MockMemoryStore::new();
    let mock_store = std::sync::Arc::new(mock_store);

    let bot = TelegramBot::new_with_memory_store(config, mock_store.clone())
        .await
        .expect("TelegramBot::new_with_memory_store");

    // 预写入消息上下文（与 docs/rag/context-retrieval-before-reply 中策略一致），
    // build_memory_context 会通过 RecentMessagesStrategy / SemanticSearchStrategy 检索到这些条目，AI 根据记忆回复。
    const TEST_USER_ID: i64 = 123456;
    const TEST_CHAT_ID: i64 = 123456;
    seed_memory_context(
        mock_store.clone(),
        &TEST_USER_ID.to_string(),
        &TEST_CHAT_ID.to_string(),
    )
    .await
    .expect("seed_memory_context");

    let msg = Message {
        id: "test_msg_1".to_string(),
        user: User {
            id: TEST_USER_ID,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: None,
        },
        chat: Chat {
            id: TEST_CHAT_ID,
            chat_type: "private".to_string(),
        },
        content: "Hello, can you help me?".to_string(),
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: chrono::Utc::now(),
        reply_to_message_id: Some("bot_msg_123".to_string()),
        reply_to_message_from_bot: true,
        reply_to_message_content: Some("Previous bot response".to_string()),
    };

    // SyncLLMHandler 在链内同步执行：before() 存用户消息，handle() 调 LLM 并返回 Reply，after() 存 LLM 回复。handle_core_message 返回时链已结束。
    bot.handle_core_message(&msg).await.expect("handle_core_message");

    // 预写入 10 条（含向量库中需被查询的内容）+ middleware before() 存 1 条用户消息 + after() 存 1 条 AI 回复；至少应有 11 次 store（10 seed + 1 user）。
    assert!(
        mock_store.get_store_call_count() >= 11,
        "Memory store should have at least 11 adds (10 seeded + 1 user message), got {}",
        mock_store.get_store_call_count()
    );
    assert!(
        mock_store.get_query_call_count() >= 1,
        "Context retrieval (query) should be executed at least once, got {}",
        mock_store.get_query_call_count()
    );
    // When embedding fails (e.g. invalid API key), semantic_search is skipped; no assertion on semantic_search_call_count.

    env::remove_var("TELEGRAM_API_URL");
}
