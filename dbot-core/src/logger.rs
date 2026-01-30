//! Logging initialization: both console and file use tracing_subscriber's fmt layer with full format (level, target, span, all fields).

use std::fs::OpenOptions;
use std::io;
use std::sync::Arc;

use tracing_subscriber::{
    fmt::format::FmtSpan,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// Initializes the global tracing subscriber.
/// Console and log file both use the fmt layer with full format (level, target, span, all fields); output is identical.
/// Uses a tee to write the same output to stdout and the log file.
/// Log level is read from the `RUST_LOG` environment variable (e.g. info, debug, trace); defaults to info if unset.
/// Load `.env` (e.g. `dotenvy::dotenv()`) before calling this so `RUST_LOG` takes effect.
pub fn init_tracing(log_file_path: &str) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;
    let file = Arc::new(file);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Single fmt layer (full format) teed to stdout and file
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
