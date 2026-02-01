//! Binary for langgraph-bot: seed messages into langgraph SqliteSaver checkpoint.
//!
//! Uses `langgraph_bot::load` and `langgraph_bot::checkpoint`; see `cli.rs` for CLI definition.

use anyhow::Result;
use clap::Parser;
use langgraph_bot::{
    create_react_runner, get_messages_from_checkpointer, import_messages_into_checkpointer,
    load_messages_from_path_with_stats, run_chat, seed_messages_to_messages_with_stats,
    verify_messages_format, verify_messages_integrity,
};
use seed_messages::generate_messages;
use std::io::{self, Write};

mod cli;

use cli::{Cli, Commands};

const DEFAULT_THREAD_ID: &str = "default";

/// Prints help message for interactive chat commands.
fn print_help() {
    println!("Available commands:");
    println!("  /help    - Show this help message");
    println!("  /exit    - Exit the chat");
    println!("  /quit    - Exit the chat");
    println!("  Any other text will be sent to the ReAct agent.");
}

/// Interactive chat loop: optional first message, then read lines from stdin until EOF or /exit.
/// Supports commands: /help, /exit, /quit. Ctrl+C also exits (SIGINT).
async fn run_chat_loop(db: &std::path::Path, first_message: Option<String>) -> Result<()> {
    let runner = create_react_runner(db).await?;
    
    println!("ReAct Chat (type /help for commands, /exit to quit)");
    println!();
    
    if let Some(m) = first_message {
        println!("> {}", m);
        match run_chat(&runner, DEFAULT_THREAD_ID, &m).await {
            Ok(reply) => println!("{}", reply),
            Err(e) => eprintln!("Error: {}", e),
        }
        println!();
    }
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        let n = io::stdin().read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Handle commands
        match line {
            "/help" => {
                print_help();
                continue;
            }
            "/exit" | "/quit" => {
                println!("Goodbye!");
                break;
            }
            _ => {}
        }
        
        // Send message to ReAct agent with retry on error
        match run_chat(&runner, DEFAULT_THREAD_ID, line).await {
            Ok(reply) => println!("{}", reply),
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!("(You can continue chatting or type /exit to quit)");
            }
        }
        println!();
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Chat { message, db } => run_chat_loop(&db, message).await?,
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
