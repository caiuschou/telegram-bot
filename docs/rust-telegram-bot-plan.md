# Rust Telegram Bot 开发方案

## 1. 核心技术栈

### 主要库推荐
- **teloxide** - 功能完整的 Telegram Bot API Rust 框架
  - 提供类型安全的 API
  - 支持异步处理
  - 内置中间件系统
  - 丰富的示例和文档
- **tokio** - 异步运行时
- **serde** / **serde_json** - JSON 序列化/反序列化

### 依赖示例
```toml
[dependencies]
teloxide = { version = "0.12", features = ["macros"] }
tokio = { version = "1.35", features = ["full"] }
serde_json = "1.0"
```

## 2. 项目结构

```
telegram-bot/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── handlers.rs    # 消息处理逻辑
│   ├── models.rs      # 数据模型
│   ├── services.rs    # 业务逻辑服务
│   ├── middleware.rs  # 中间件
│   └── utils.rs       # 工具函数
└── README.md
```

## 3. 核心功能模块

### 3.1 基础 Bot 初始化
```rust
use teloxide::{prelude::*, dispatching::dialogue::*};
use std::sync::Arc;

type MyDialogue = Dialogue<State, UpdateHandler>;

#[derive(Clone, Default)]
enum State {
    #[default]
    Start,
}

#[tokio::main]
async fn main() {
    let bot = Bot::from_env().auto_send();

    let dispatcher = Dispatcher::builder(bot, Default::route())
        .dependencies(Default::dependencies)
        .build()
        .await
        .expect("Can't build dispatcher");

    dispatcher.run().await;
}
```

### 3.2 消息处理
```rust
use teloxide::requests::Requester;

async fn command_start(cx: UpdateIn<Ctx, MyDialogue>) {
    let bot = cx.bot();
    cx.answer("欢迎使用 Telegram Bot!")
        .await
        .expect("Can't send message");
    cx.next().await;
}
```

### 3.3 回复机器人
```rust
async fn echo(cx: UpdateIn<Ctx, MyDialogue>) {
    if let Update::Message(msg) = cx.update {
        cx.answer(&msg.text.unwrap())
            .await
            .expect("Can't send reply");
    }
    cx.next().await;
}
```

### 3.4 命令处理器
```rust
use teloxide::requests::Requester;

async fn command_help(cx: UpdateIn<Ctx, MyDialogue>) {
    let help_text = "可用命令:
/start - 开始对话
/help - 显示帮助
/echo <text> - 回复消息
";

    cx.answer(help_text)
        .await
        .expect("Can't send help text");
    cx.next().await;
}
```

## 4. 中间件系统

### 4.1 日志中间件
```rust
async fn logging_middleware(cx: MiddlewareNext<Ctx>) -> Response {
    let update = cx.update.clone();
    log::info!("收到更新: {:?}", update);
    cx.next().await
}
```

### 4.2 验证中间件
```rust
async fn auth_middleware(cx: MiddlewareNext<Ctx>) -> Response {
    let update = cx.update.clone();

    if let Update::Message(msg) = &update {
        if msg.chat.id != ALLOWED_CHAT_ID {
            return cx.answer("未授权访问").await;
        }
    }

    cx.next().await
}
```

## 5. 业务逻辑服务

### 5.1 用户状态管理
```rust
use std::collections::HashMap;

#[derive(Clone)]
pub struct UserService {
    user_states: Arc<Mutex<HashMap<i64, State>>>,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            user_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_state(&self, user_id: i64, state: State) {
        self.user_states.lock().unwrap().insert(user_id, state);
    }

    pub fn get_state(&self, user_id: i64) -> Option<State> {
        self.user_states.lock().unwrap().get(&user_id).cloned()
    }
}
```

### 5.2 数据库集成
```rust
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct DatabaseService {
    pool: SqlitePool,
}

impl DatabaseService {
    pub async fn new(database_url: &str) -> Self {
        let pool = SqlitePool::connect(database_url).await.unwrap();
        Self { pool }
    }

    pub async fn store_message(&self, user_id: i64, text: &str) {
        sqlx::query(
            "INSERT INTO messages (user_id, text) VALUES (?, ?)"
        )
        .bind(user_id)
        .bind(text)
        .execute(&self.pool)
        .await
        .unwrap();
    }
}
```

## 6. Webhook vs Polling

### 6.1 Polling 方式
```rust
async fn main() {
    let bot = Bot::from_env().auto_send();
    let dispatcher = Dispatcher::builder(bot, Default::route())
        .dependencies(Default::dependencies)
        .build()
        .await
        .expect("Can't build dispatcher");

    dispatcher.run().await;
}
```

### 6.2 Webhook 方式
```rust
use teloxide::dispatching::webhooks;
use teloxide::requests::Requester;

#[tokio::main]
async fn main() {
    let bot = Bot::from_env();
    let webhook = webhooks::default::CbHttp::new(bot).path("/webhook");

    let handler = Default::route()
        .endpoint(Box::new(webhook.into_handler()));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    teloxide::repl(listener, handler).await;
}
```

## 7. 部署方案

### 7.1 Docker 部署
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/telegram-bot /app/telegram-bot
CMD ["./telegram-bot"]
```

### 7.2 systemd 服务
```ini
[Unit]
Description=Telegram Bot Service
After=network.target

[Service]
Type=simple
User=bot
WorkingDirectory=/opt/telegram-bot
ExecStart=/opt/telegram-bot/telegram-bot
Restart=always

[Install]
WantedBy=multi-user.target
```

## 8. 监控和日志

### 8.1 日志配置
```rust
use log::{info, warn, error};
use env_logger::Env;

fn init_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}
```

### 8.2 性能监控
```rust
use prometheus::{Counter, Registry};

struct Metrics {
    requests: Counter,
}

impl Metrics {
    fn new(registry: &Registry) -> Self {
        let requests = Counter::new("bot_requests_total", "Total bot requests").unwrap();
        registry.register(Box::new(requests.clone())).unwrap();
        Self { requests }
    }
}
```

## 9. 安全考虑

### 9.1 环境变量管理
```rust
use dotenv::dotenv;
use std::env;

pub fn init_env() {
    dotenv().ok();
    env::var("BOT_TOKEN").expect("BOT_TOKEN environment variable not set");
}
```

### 9.2 防止重放攻击
```rust
async fn replay_attack_middleware(cx: MiddlewareNext<Ctx>) -> Response {
    let update = cx.update.clone();

    if let Update::Message(msg) = &update {
        // 实现你的验证逻辑
        if is_duplicate_message(&msg).await {
            return cx.answer("重复消息,请勿重复发送").await;
        }
    }

    cx.next().await
}
```

## 10. 测试策略

### 10.1 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_service() {
        let service = UserService::new();
        service.set_state(123, State::Start);
        assert_eq!(service.get_state(123), Some(State::Start));
    }
}
```

### 10.2 集成测试
```rust
#[tokio::test]
async fn test_bot_reply() {
    let bot = Bot::from_env().auto_send();
    let response = bot.send_message(123456, "Test message")
        .await
        .expect("Can't send message");

    assert_eq!(response.text().unwrap(), "Test message");
}
```

## 11. 扩展功能

### 11.1 语音消息处理
```rust
async fn handle_voice(cx: UpdateIn<Ctx, MyDialogue>) {
    if let Update::Message(msg) = cx.update {
        if let Some(voice) = msg.voice {
            cx.answer("收到语音消息!")
                .await
                .expect("Can't send response");
        }
    }
    cx.next().await;
}
```

### 11.2 文件上传
```rust
async fn upload_file(cx: UpdateIn<Ctx, MyDialogue>) {
    if let Update::Message(msg) = cx.update {
        let file_id = "your_file_id";

        cx.send_document(telegram_types::request::SendDocument {
            chat_id: msg.chat.id,
            document: FileId(file_id.to_string()),
            caption: Some("文件描述".to_string()),
            parse_mode: None,
        })
        .await
        .expect("Can't send file");
    }
    cx.next().await;
}
```

## 12. 常见问题解决

### 12.1 消息队列处理
```rust
use redis::AsyncCommands;

async fn send_long_task(bot: Bot, user_id: i64, task_id: String) {
    let mut conn = redis::Client::open("redis://localhost/").unwrap()
        .get_async_connection()
        .await
        .unwrap();

    conn.set(task_id.as_str(), "processing").await.unwrap();

    // 执行耗时任务
    do_expensive_work().await;

    conn.set(task_id.as_str(), "completed").await.unwrap();

    bot.send_message(user_id, "任务已完成")
        .await
        .expect("Can't send message");
}
```

### 12.2 错误处理
```rust
async fn robust_handler(cx: UpdateIn<Ctx, MyDialogue>) {
    match cx.update {
        Update::Message(msg) => {
            if let Err(e) = handle_message(msg).await {
                cx.answer(format!("处理失败: {:?}", e))
                    .await
                    .ok();
            }
        }
        _ => {}
    }
    cx.next().await;
}
```

## 13. 性能优化

### 13.1 批量处理
```rust
async fn batch_process_messages(messages: Vec<Message>) {
    let mut batch = Vec::new();
    for msg in messages {
        batch.push(process_message(msg).await);
        if batch.len() >= 10 {
            futures::future::join_all(batch).await;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        futures::future::join_all(batch).await;
    }
}
```

### 13.2 连接池优化
```rust
let pool = SqlitePool::builder()
    .max_connections(20)
    .min_connections(5)
    .connect(database_url)
    .await
    .unwrap();
```

## 14. 开发工具

### 14.1 调试工具
```bash
# 启动开发环境
cargo run

# 查看日志
RUST_LOG=debug cargo run

# 运行测试
cargo test

# 格式化代码
cargo fmt

# 检查代码
cargo clippy
```

### 14.2 性能分析
```bash
# 启动性能分析
cargo flamegraph

# 运行基准测试
cargo bench
```

## 15. 最佳实践总结

1. 使用环境变量管理敏感信息
2. 实现完善的错误处理机制
3. 添加适当的日志记录
4. 使用中间件分离关注点
5. 考虑并发和性能优化
6. 编写单元测试和集成测试
7. 使用 Docker 等容器化部署
8. 定期监控和更新依赖
9. 实现优雅的降级机制
10. 遵循 Rust 的所有权和借用规则
