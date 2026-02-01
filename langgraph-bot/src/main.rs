//! Binary for langgraph-bot: seed messages into langgraph SqliteSaver checkpoint.
//!
//! Uses `langgraph_bot::load` and `langgraph_bot::checkpoint`; see `cli.rs` for CLI definition.

use anyhow::Result;
use clap::Parser;
use langgraph_bot::{
    get_messages_from_checkpointer, import_messages_into_checkpointer,
    load_messages_from_path_with_stats, seed_messages_to_messages_with_stats,
    verify_messages_format, verify_messages_integrity,
};
use seed_messages::generate_messages;

mod cli;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Seed {
            messages,
            db,
            thread_id,
        } => {
            let (messages, skipped) = match messages {
                Some(path) => load_messages_from_path_with_stats(&path)?,
                None => seed_messages_to_messages_with_stats(generate_messages()?),
            };
            if skipped > 0 {
                eprintln!("Warning: {} messages skipped (direction not received/sent)", skipped);
            }
            let thread_id = thread_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let id = import_messages_into_checkpointer(&db, &thread_id, &messages).await?;
            println!("Seeded thread {} with checkpoint id: {}", thread_id, id);

            let verified = get_messages_from_checkpointer(&db, &thread_id).await?;
            verify_messages_integrity(&messages, &verified)?;
            verify_messages_format(&verified)?;
            println!("Integrity: OK ({} messages)", verified.len());
            println!("Format: OK (User/Assistant only)");

            for (i, msg) in verified.iter().take(3).enumerate() {
                let preview = match msg {
                    langgraph::Message::User(s) => {
                        format!("User: {}", s.chars().take(40).collect::<String>())
                    }
                    langgraph::Message::Assistant(s) => {
                        format!("Assistant: {}", s.chars().take(40).collect::<String>())
                    }
                    _ => "Other".into(),
                };
                println!("  [{}] {}", i + 1, preview);
            }
            if verified.len() > 3 {
                println!("  ... and {} more", verified.len() - 3);
            }
        }
    }

    Ok(())
}
