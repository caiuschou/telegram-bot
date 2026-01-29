//! 日志初始化：控制台与文件均使用 tracing_subscriber 的 fmt layer 完整格式（级别、target、span、所有字段）。

use std::fs::OpenOptions;
use std::io;
use std::sync::Arc;

use tracing_subscriber::{
    fmt::format::FmtSpan,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// 初始化全局 tracing 订阅者。
/// 控制台与日志文件均使用 fmt layer 的完整格式（级别、target、span、所有字段），输出一致。
/// 通过 Tee 将同一份输出同时写入 stdout 与日志文件。
/// 从环境变量 RUST_LOG 读取日志级别（如 info、debug、trace）；未设置则默认为 info。
/// 注意：需在调用本函数前加载 .env（如 dotenvy::dotenv()），否则 RUST_LOG 不会生效。
pub fn init_tracing(log_file_path: &str) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;
    let file = Arc::new(file);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // 信息更多的 layer：fmt 完整格式（级别、target、span、所有字段），同时写入控制台与文件
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let writer = io::stdout.and(file);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_file(false)
        .with_line_number(false);

    Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to set global subscriber: {}", e))?;

    Ok(())
}
