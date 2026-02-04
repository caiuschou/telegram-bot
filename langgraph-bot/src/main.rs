//! Binary for langgraph-bot: load or seed messages into langgraph SqliteSaver checkpoint.
//!
//! Uses `langgraph_bot::load` and `langgraph_bot::checkpoint`; see `cli.rs` for CLI definition.

use anyhow::{Context, Result};
use clap::Parser;
use langgraph_bot::{
    create_react_runner, format_thread_summary,
    get_messages_from_checkpointer, get_react_state_from_checkpointer,
    import_messages_into_checkpointer, list_thread_ids,
    load_all_messages_from_telegram_db, load_messages_from_path_with_user_info_with_stats,
    load_messages_from_telegram_db,
    print_runtime_info, run_chat, run_chat_stream,
    seed_messages_to_messages_with_user_info_with_stats, verify_messages_format,
    verify_messages_integrity,
};
use seed_messages::generate_messages;
use std::io::{self, Write};
use std::path::PathBuf;

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
async fn run_chat_loop(
    db: &std::path::Path,
    first_message: Option<String>,
    stream: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    let runner = create_react_runner(db).await?;

    println!("ReAct Chat (type /help for commands, /exit to quit)");
    println!();

    if let Some(m) = first_message {
        println!("> {}", m);
        let result = if stream {
            run_chat_stream(&runner, DEFAULT_THREAD_ID, &m, |chunk| {
                print!("{}", chunk);
                let _ = io::stdout().flush();
            }, None)
            .await
        } else {
            run_chat(&runner, DEFAULT_THREAD_ID, &m, None).await
        };
        match result {
            Ok(reply) => {
                if stream {
                    println!();
                } else {
                    println!("{}", reply);
                }
            }
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
        let result = if stream {
            run_chat_stream(&runner, DEFAULT_THREAD_ID, line, |chunk| {
                print!("{}", chunk);
                let _ = io::stdout().flush();
            }, None)
            .await
        } else {
            run_chat(&runner, DEFAULT_THREAD_ID, line, None).await
        };
        match result {
            Ok(reply) => {
                if stream {
                    println!();
                } else {
                    println!("{}", reply);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!("(You can continue chatting or type /exit to quit)");
            }
        }
        println!();
    }
    Ok(())
}

const ENV_MESSAGES_PATH: &str = "LANGGRAPH_MESSAGES_PATH";
const ENV_TELEGRAM_MESSAGES_DB: &str = "TELEGRAM_MESSAGES_DB";
const MEMORY_PREVIEW_LEN: usize = 50;

/// When -m is not set: if TELEGRAM_MESSAGES_DB is set, load from that SQLite; else LANGGRAPH_MESSAGES_PATH (JSON).
/// With Telegram DB: -t given → load that chat_id only, resolved_thread_id = that id; -t omitted → load all messages, resolved_thread_id = Some("all").
/// Returns (messages, skipped, resolved_thread_id). Caller uses resolved_thread_id as thread_id when present.
fn resolve_messages_source(
    messages_path: Option<PathBuf>,
    thread_id_for_db: Option<&str>,
) -> Result<(Vec<langgraph::Message>, usize, Option<String>)> {
    if let Some(p) = messages_path {
        let (m, skipped) = load_messages_from_path_with_user_info_with_stats(&p)
            .with_context(|| format!("Load messages from {}", p.display()))?;
        return Ok((m, skipped, None));
    }
    let telegram_db = std::env::var(ENV_TELEGRAM_MESSAGES_DB)
        .ok()
        .filter(|s| !s.trim().is_empty());
    if let Some(ref db_path) = telegram_db {
        let (m, skipped) = if let Some(t) = thread_id_for_db {
            let chat_id = t.trim().parse::<i64>().with_context(|| {
                format!("When using -t with Telegram DB, thread_id must be numeric (chat_id), got {:?}", t)
            })?;
            let (messages, sk) = load_messages_from_telegram_db(db_path, chat_id, None)
                .with_context(|| format!("Load from Telegram DB {}", db_path))?;
            (messages, sk)
        } else {
            // Not passing -t: import all messages from the DB into one thread.
            load_all_messages_from_telegram_db(db_path, None)
                .with_context(|| format!("Load from Telegram DB {}", db_path))?
        };
        let resolved = if thread_id_for_db.is_none() {
            Some("all".to_string())
        } else {
            thread_id_for_db.map(String::from)
        };
        return Ok((m, skipped, resolved));
    }
    let path = std::env::var(ENV_MESSAGES_PATH)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .with_context(|| format!("Set -m/--messages, or {}, or {} in .env", ENV_TELEGRAM_MESSAGES_DB, ENV_MESSAGES_PATH))?;
    let (m, skipped) = load_messages_from_path_with_user_info_with_stats(&path)
        .with_context(|| format!("Load messages from {}", path.display()))?;
    Ok((m, skipped, None))
}

/// Prints short-term memory (checkpoint) summary: either one thread or all threads with message count and previews.
async fn print_memory_summary(
    db: &std::path::Path,
    thread_id: Option<&str>,
) -> Result<()> {
    println!("Short-term memory (checkpoint): {}", db.display());
    if let Some(tid) = thread_id {
        let state = get_react_state_from_checkpointer(db, tid).await?;
        println!("{}\n", format_thread_summary(tid, &state, MEMORY_PREVIEW_LEN));
        return Ok(());
    }
    let ids = list_thread_ids(db)?;
    if ids.is_empty() {
        println!("  (no threads)");
        return Ok(());
    }
    println!("  threads: {}", ids.len());
    for tid in &ids {
        let state = get_react_state_from_checkpointer(db, tid).await?;
        println!("{}\n", format_thread_summary(tid, &state, MEMORY_PREVIEW_LEN));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { db } => print_runtime_info(&db).await?,
        Commands::Memory { db, thread_id } => print_memory_summary(&db, thread_id.as_deref()).await?,
        Commands::Chat { message, db, stream, verbose } => run_chat_loop(&db, message, stream, verbose).await?,
        Commands::Load { messages, db, thread_id } => {
            let (messages, skipped, resolved_thread_id) =
                resolve_messages_source(messages, thread_id.as_deref())?;
            if skipped > 0 {
                eprintln!("Warning: {} messages skipped (direction not received/sent)", skipped);
            }
            let thread_id = resolved_thread_id
                .or(thread_id)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let id = import_messages_into_checkpointer(&db, &thread_id, &messages).await?;
            println!("Loaded thread {} with checkpoint id: {}", thread_id, id);
            print_import_preview(&db, &thread_id, &messages).await?;
        }
        Commands::Seed { db, thread_id } => {
            let (messages, skipped) =
                seed_messages_to_messages_with_user_info_with_stats(generate_messages()?);
            if skipped > 0 {
                eprintln!("Warning: {} messages skipped (direction not received/sent)", skipped);
            }
            let thread_id = thread_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let id = import_messages_into_checkpointer(&db, &thread_id, &messages).await?;
            println!("Seeded thread {} with checkpoint id: {}", thread_id, id);
            print_import_preview(&db, &thread_id, &messages).await?;
        }
    }

    Ok(())
}

async fn print_import_preview(
    db: &std::path::Path,
    thread_id: &str,
    messages: &[langgraph::Message],
) -> Result<()> {
    let verified = get_messages_from_checkpointer(db, thread_id).await?;
    verify_messages_integrity(messages, &verified)?;
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
    Ok(())
}
