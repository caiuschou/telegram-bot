//! CLI for langgraph-bot binary.
//!
//! Parses subcommands and args; see `main.rs` for dispatch to `langgraph_bot::load` and
//! `langgraph_bot::checkpoint`.

use clap::{Parser, Subcommand};

/// Root CLI: holds a single subcommand. Parsed by `main.rs` and matched to load/checkpoint calls.
#[derive(Parser)]
#[command(name = "langgraph-bot")]
#[command(about = "Seed messages into langgraph short-term memory (SqliteSaver checkpoint)")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands; `Seed` is the only variant and is handled in `main.rs` via load + checkpoint.
#[derive(Subcommand)]
pub enum Commands {
    /// Write messages into SqliteSaver checkpoint for a thread. Messages from seed-messages (default) or from a JSON file.
    Seed {
        /// Path to messages JSON (same shape as seed-messages). If omitted, uses seed-messages::generate_messages().
        #[arg(short, long)]
        messages: Option<std::path::PathBuf>,

        /// Path to Sqlite checkpoint database (created if missing).
        #[arg(short, long, default_value = "checkpoint.db")]
        db: std::path::PathBuf,

        /// Thread ID for the conversation. If omitted, a new UUID is generated.
        #[arg(short, long)]
        thread_id: Option<String>,
    },
}
