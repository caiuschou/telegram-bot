//! runner 集成测试工具
//!
//! - 对 `telegram-bot/src/runner.rs` 中 `run_bot` 的集成测试提供基础工具。
//! - 与外部交互：
//!   - 通过 `.env.test` / `.env` 加载真实/测试配置（OPENAI_API_KEY 等）。
//!   - 使用临时目录作为数据库与日志路径，避免污染工作区。
//!   - 通过 `MockMemoryStore` 在测试中替代真实向量存储。

use std::env;
use std::sync::Once;

use chrono::Utc;
use telegram_bot::BotConfig;
use tempfile::TempDir;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;

mod mock_memory_store;
use mock_memory_store::MockMemoryStore;

use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use memory_lance::LanceVectorStore;

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

/// 主流程集成测试占位：AI 回复完整流程
///
/// 后续将根据 `docs/TELEGRAM_BOT_TEST_PLAN.md` 中的
/// "AI 回复完整流程" 场景，补充：
/// - Mock Telegram getMe / sendMessage
/// - Mock OpenAI ChatCompletion
/// - 启动 `run_bot` 并模拟用户消息
/// - 验证消息持久化、记忆写入与查询、AI 回复发送等关键步骤
#[tokio::test]
async fn test_ai_reply_complete_flow_smoke() {
    init_tracing();

    // 当前版本仅验证测试环境和基础组件可以正常初始化，
    // 避免主流程测试在未完全实现前导致编译失败。
    let temp_dir = TempDir::new().expect("TempDir::new must succeed");
    env::set_var("OPENAI_API_KEY", "test_key_for_integration_flow");

    let _config = setup_test_config(&temp_dir);
    let _memory_store = MockMemoryStore::new();

    // TODO:
    // - 使用 `mock_telegram_get_me` / `mock_telegram_send_message` 与 Telegram 通讯逻辑打通。
    // - 使用可注入 MemoryStore 的 TelegramBot 构造函数与 MockMemoryStore 计数器，驱动并验证完整 AI 流程（3.x / 3.4）。
}

/// Lance 向量存储验证测试
///
/// 验证点：
/// - Lance 数据库初始化 - 创建数据库连接、初始化向量表
/// - 数据库目录被正确创建
/// - 消息向量化存储到 Lance（添加 MemoryEntry）
/// - 数据持久化验证
///
/// 外部交互：
/// - 在临时目录创建 Lance 数据库
/// - 创建并写入 MemoryEntry 到数据库
/// - 验证数据库文件存在且可读取
#[tokio::test]
async fn test_lance_vector_store_verification() {
    init_tracing();

    // 1. 创建临时目录用于 Lance 数据库
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lance_db_path = temp_dir.path().join("lance_db");
    let lance_path_str = lance_db_path.to_string_lossy().to_string();

    info!(lance_path = %lance_path_str, "Creating Lance vector store");

    // 2. 验证数据库目录在创建前不存在
    assert!(
        !lance_db_path.exists(),
        "Lance database directory should not exist before creation"
    );

    // 3. 创建 LanceVectorStore（会自动初始化数据库和表）
    let store = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to create LanceVectorStore");

    // 4. 验证数据库目录被正确创建
    assert!(
        lance_db_path.exists(),
        "Lance database directory should be created after store initialization"
    );

    // 5. 验证数据库目录包含文件
    let lance_db_files =
        std::fs::read_dir(&lance_db_path).expect("Should be able to read Lance database directory");
    let file_count = lance_db_files.count();
    assert!(
        file_count > 0,
        "Lance database should contain data files, found {} files",
        file_count
    );

    // 6. 创建测试 MemoryEntry（包含模拟的 embedding 向量）
    let test_content = "Test message for Lance vector store";
    let test_embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();

    let metadata = MemoryMetadata {
        user_id: Some("test_user_123".to_string()),
        conversation_id: Some("test_conversation_456".to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(1.0),
    };

    let entry = MemoryEntry {
        id: Uuid::new_v4(),
        content: test_content.to_string(),
        embedding: Some(test_embedding),
        metadata,
    };

    // 7. 添加 MemoryEntry 到 Lance 存储
    store
        .add(entry.clone())
        .await
        .expect("Failed to add entry to Lance store");

    // 8. 验证 MemoryEntry 可以被检索
    let retrieved = store
        .get(entry.id)
        .await
        .expect("Failed to get entry from Lance store");

    assert!(
        retrieved.is_some(),
        "MemoryEntry should be retrievable after adding"
    );

    let retrieved_entry = retrieved.unwrap();
    assert_eq!(
        retrieved_entry.id, entry.id,
        "Retrieved entry should have the same ID"
    );
    assert_eq!(
        retrieved_entry.content, entry.content,
        "Retrieved entry should have the same content"
    );
    assert!(
        retrieved_entry.embedding.is_some(),
        "Retrieved entry should have embedding"
    );

    // 9. 验证用户搜索功能
    let user_entries = store
        .search_by_user("test_user_123")
        .await
        .expect("Failed to search by user");

    assert!(
        !user_entries.is_empty(),
        "Should find entries for test_user_123"
    );
    assert!(
        user_entries.iter().any(|e| e.id == entry.id),
        "Search results should include the added entry"
    );

    // 10. 验证会话搜索功能
    let conversation_entries = store
        .search_by_conversation("test_conversation_456")
        .await
        .expect("Failed to search by conversation");

    assert!(
        !conversation_entries.is_empty(),
        "Should find entries for test_conversation_456"
    );
    assert!(
        conversation_entries.iter().any(|e| e.id == entry.id),
        "Search results should include the added entry"
    );

    // 11. 验证语义搜索功能
    let query_embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
    let search_results = store
        .semantic_search(&query_embedding, 10)
        .await
        .expect("Failed to perform semantic search");

    assert!(
        !search_results.is_empty(),
        "Semantic search should return results"
    );

    // 12. 清理 - TempDir 会在作用域结束时自动删除临时目录
    // 验证数据持久化：重新打开数据库并读取数据
    let store2 = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to reopen LanceVectorStore");

    let retrieved_after_reopen = store2
        .get(entry.id)
        .await
        .expect("Failed to get entry after reopening store");

    assert!(
        retrieved_after_reopen.is_some(),
        "MemoryEntry should be retrievable after reopening the store (data persistence)"
    );

    info!("Lance vector store verification test completed successfully");
}
