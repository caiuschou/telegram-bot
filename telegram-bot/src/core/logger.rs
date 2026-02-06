//! Logging initialization: human-readable format (timestamp, level, message, fields) to both console and file.

use std::fs::OpenOptions;
use std::io;
use std::sync::Arc;

use tracing_subscriber::{
    fmt::format::{FmtSpan, Writer},
    fmt::time::FormatTime,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// Local time in `YYYY-MM-DD HH:MM:SS` for human-readable log lines.
struct ChronoLocal;

impl FormatTime for ChronoLocal {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let t = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        write!(w, "{} ", t)
    }
}

/// Initializes the global tracing subscriber.
///
/// Output is human-readable: `YYYY-MM-DD HH:MM:SS LEVEL [target] message key=value ...`
/// Teed to stdout and the given log file. No ANSI codes so the log file is plain text.
/// Log level from `RUST_LOG` (e.g. `info`, `debug`); default `info`. Load `.env` before calling.
pub fn init_tracing(log_file_path: &str) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;
    let file = Arc::new(file);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let writer = io::stdout.and(file);

    let event_format = tracing_subscriber::fmt::format()
        .with_timer(ChronoLocal)
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .event_format(event_format)
        .with_span_events(FmtSpan::NONE)
        .with_ansi(false);

    Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to set global subscriber: {}", e))?;

    Ok(())
}
