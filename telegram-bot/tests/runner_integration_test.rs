//! Integration tests for telegram-bot runner (run_bot, TelegramBot, handle_core_message).
//!
//! Loads config from `.env.test` or `.env`; uses temp dirs for DB and logs; uses MockMemoryStore instead of real vector store.
//! Pre-seeded memory entries are retrieved by context strategies when building LLM context.

use std::env;
use std::sync::Once;

use telegram_bot::{Chat, Message, MessageDirection, User};
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
        // Prefer in-memory store in integration tests to avoid hard dependency on Lance/SQLite
        env::set_var("MEMORY_STORE_TYPE", "memory");
    }

    // Integration tests use OpenAI embedding to avoid BIGMODEL_API_KEY dependency
    env::set_var("EMBEDDING_PROVIDER", "openai");

    // Always override path config for tests
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

/// Initialize tracing; call once per test process.
///
/// - Use `RUST_LOG` / `RUST_LOG_STYLE` env vars to control level and style.
/// - `with_test_writer()` ensures log output goes to test console when running `cargo test`.
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

/// Teloxide request path format is `/bot<token>/<method>`; test token is `test_bot_token_12345`.
const TEST_BOT_TOKEN: &str = "test_bot_token_12345";

/// Register mocks for Telegram getMe and sendMessage on the mock server.
/// Paths must match teloxide requests: `/bot<token>/getMe`, `/bot<token>/sendMessage`.
/// Returns mock guards; caller must hold until request completes, else server returns empty body and JSON parse fails.
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

/// Pre-seed MemoryStore with message context for integration tests (reply-from-memory).
///
/// Aligned with strategies in `build_memory_context` (see `docs/rag/context-retrieval-before-reply.md`):
/// - **RecentMessagesStrategy**: fetch recent messages by `conversation_id` / `user_id`;
/// - **SemanticSearchStrategy**: vector search on current question (embed + semantic_search),
///   requires store entries to query; this function writes the content that will be queried.
/// Uses same `user_id` / `conversation_id` as test messages (e.g. "123456") so strategies can retrieve them;
/// AI reply will include this context.
///
/// External: only calls `MemoryStore::add`, no network or file I/O.
async fn seed_memory_context(
    store: std::sync::Arc<dyn MemoryStore>,
    user_id: &str,
    conversation_id: &str,
) -> Result<(), anyhow::Error> {
    let user_id = Some(user_id.to_string());
    let conversation_id = Some(conversation_id.to_string());
    let base_time = chrono::Utc::now() - chrono::Duration::hours(1);

    // Conversation history (RecentMessagesStrategy fetches by conversation)
    let entries: &[(&str, MemoryRole, i64)] = &[
        ("My name is Xiao Ming.", MemoryRole::User, 0),
        ("Hi Xiao Ming, how can I help?", MemoryRole::Assistant, 1),
        ("I'd like to know about the weather.", MemoryRole::User, 2),
        ("Sure, you can ask me for a city or date.", MemoryRole::Assistant, 3),
        // Content for vector store: user preferences, past help, timezone, for SemanticSearchStrategy
        ("I usually prefer to communicate in English.", MemoryRole::User, 4),
        ("OK, I'll reply in English by default.", MemoryRole::Assistant, 5),
        ("Last time you looked up Beijing weather for me, thanks.", MemoryRole::User, 6),
        ("You're welcome, ask anytime for weather again.", MemoryRole::Assistant, 7),
        ("Please remember: my timezone is UTC+8.", MemoryRole::User, 8),
        ("Noted, I'll consider UTC+8 in time-related replies.", MemoryRole::Assistant, 9),
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

/// Smoke test: LLM reply flow (env and component init only)
#[tokio::test]
async fn test_llm_reply_complete_flow_smoke() {
    init_tracing();
    env::set_var("EMBEDDING_PROVIDER", "openai");

    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    env::set_var("OPENAI_API_KEY", "test_key_for_integration_flow");

    let _config = setup_test_config(&temp_dir);
    let _memory_store = MockMemoryStore::new();
}

/// E2E test: AI reply flow (requires real OPENAI_API_KEY; Telegram API uses mock).
///
/// Checks:
/// - TelegramBot init with MockMemoryStore; Telegram requests go to mock server (no real Telegram)
/// - User "reply to bot" message triggers AI pipeline
/// - After handle_core_message: persistence, memory write, query are invoked
/// - After AI handler: store at least 1 call, query at least 1 call
///
/// External: uses OPENAI_API_KEY for real OpenAI API; skipped if unset; Telegram goes to local mock.
#[tokio::test]
async fn test_ai_reply_complete_flow() {
    init_tracing();
    env::set_var("EMBEDDING_PROVIDER", "openai");

    // Load .env.test / .env first, then check OPENAI_API_KEY, else key from file won't be read
    let _ = dotenvy::from_filename(".env.test").or_else(|_| dotenvy::dotenv());

    if env::var("OPENAI_API_KEY").is_err() {
        eprintln!("SKIP: OPENAI_API_KEY not set, skipping LLM reply E2E test");
        return;
    }

    // Start Telegram API mock server to avoid Invalid bot token from fake token (new_async avoids block_on inside tokio)
    let mut server = mockito::Server::new_async().await;
    // Hold mock guards until test end; else after drop server returns empty body and teloxide JSON parse gets EOF
    let (_mock_get_me, _mock_send) = register_telegram_mocks(&mut server);
    env::set_var("TELEGRAM_API_URL", server.url());

    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    let config = setup_test_config(&temp_dir);
    let mock_store = MockMemoryStore::new();
    let mock_store = std::sync::Arc::new(mock_store);

    let handler = {
        let components = telegram_bot::build_bot_components(&config, mock_store.clone(), None)
            .await
            .expect("build_bot_components");
        dbot_llm::build_llm_handler(&config, components).expect("build_llm_handler")
    };

    let bot = TelegramBot::new_with_memory_store(config, mock_store.clone(), handler)
        .await
        .expect("TelegramBot::new_with_memory_store");

    // Pre-seed message context (aligned with docs/rag/context-retrieval-before-reply);
    // build_memory_context retrieves via RecentMessagesStrategy / SemanticSearchStrategy; AI replies from memory.
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

    // SyncLLMHandler runs in chain: before() stores user msg, handle() calls LLM and returns Reply, after() stores LLM reply. Chain done when handle_core_message returns.
    bot.handle_core_message(&msg).await.expect("handle_core_message");

    // 10 seeded entries (including vector-store content) + handler before() 1 user msg + after() 1 AI reply; at least 11 store calls (10 seed + 1 user).
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
