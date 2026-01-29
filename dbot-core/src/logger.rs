use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

use chrono::Local;

use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::format::FmtSpan,
    layer::{Context, SubscriberExt},
    EnvFilter, Layer, Registry,
};

pub struct AppLayer {
    log_file: Mutex<std::fs::File>,
}

impl<S> Layer<S> for AppLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = metadata.level();

        let mut message_string = String::new();
        let mut visitor = MessageVisitor::new(&mut message_string);
        event.record(&mut visitor);

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] [{}] {}\n", timestamp, level, message_string.trim());

        println!("{}", log_line.trim());

        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
    }
}

struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> MessageVisitor<'a> {
    fn new(message: &'a mut String) -> Self {
        Self { message }
    }
}

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.message = value.to_string();
        }
    }
}

impl AppLayer {
    pub fn new(log_file_path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file_path)?;

        Ok(Self {
            log_file: Mutex::new(file),
        })
    }
}

/// 初始化全局 tracing 订阅者。
/// 从环境变量 RUST_LOG 读取日志级别（如 info、debug、trace）；若未设置则默认为 info。不做任何额外过滤。
/// 注意：需在调用本函数前加载 .env（如 dotenvy::dotenv()），否则 RUST_LOG 不会生效。
pub fn init_tracing(log_file_path: &str) -> anyhow::Result<()> {
    let app_layer = AppLayer::new(log_file_path)?;

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(app_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_target(false)
                .with_thread_ids(true),
        );

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("Failed to set global subscriber: {}", e))?;

    Ok(())
}
