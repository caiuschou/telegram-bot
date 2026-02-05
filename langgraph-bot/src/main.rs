//! Binary for langgraph-bot: load or seed messages into langgraph SqliteSaver checkpoint.
//!
//! Uses `langgraph_bot::load` and `langgraph_bot::checkpoint`; see `cli.rs` for CLI definition.

use anyhow::Result;
use clap::Parser;
use langgraph_bot::print_runtime_info;

mod chat;
mod cli;
mod commands;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::from_filename(".env").ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { db } => print_runtime_info(&db).await?,
        Commands::Memory { db, thread_id } => {
            commands::print_memory_summary(&db, thread_id.as_deref()).await?
        }
        Commands::Chat {
            message,
            db,
            stream,
            verbose,
        } => chat::run_chat_loop(&db, message, stream, verbose).await?,
        Commands::Run { token, db } => langgraph_bot::run_telegram(&db, token).await?,
        Commands::Load {
            messages,
            db,
            thread_id,
        } => commands::cmd_load(messages, &db, thread_id).await?,
        Commands::Seed { db, thread_id } => commands::cmd_seed(&db, thread_id).await?,
    }

    Ok(())
}
