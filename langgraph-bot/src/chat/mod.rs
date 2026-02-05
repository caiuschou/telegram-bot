//! Interactive chat loop and one-turn execution for the ReAct agent.
//!
//! Used by the `chat` subcommand: optional first message, then read lines from stdin until EOF or /exit.

use anyhow::Result;
use langgraph_bot::{create_react_runner, run_chat_stream, ReactRunner};
use std::io::{self, Write};

pub const DEFAULT_THREAD_ID: &str = "default";

/// Prints help message for interactive chat commands.
pub fn print_help() {
    println!("Available commands:");
    println!("  /help    - Show this help message");
    println!("  /exit    - Exit the chat");
    println!("  /quit    - Exit the chat");
    println!("  Any other text will be sent to the ReAct agent.");
}

/// Runs one chat turn: sends `content` to the ReAct agent via `run_chat_stream`, prints reply (incremental or at end) or error.
/// When `show_continue_hint_on_error` is true, also prints a hint to continue or /exit.
/// When `stream` is true, prints each chunk as it arrives; when false, collects chunks and prints the full reply at end.
pub async fn run_one_turn(
    runner: &ReactRunner,
    thread_id: &str,
    content: &str,
    stream: bool,
    show_continue_hint_on_error: bool,
) -> Result<()> {
    let result = if stream {
        run_chat_stream(
            runner,
            thread_id,
            content,
            |chunk| {
                print!("{}", chunk);
                let _ = io::stdout().flush();
            },
            None,
        )
        .await
    } else {
        run_chat_stream(runner, thread_id, content, |_| {}, None).await
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
            if show_continue_hint_on_error {
                eprintln!("(You can continue chatting or type /exit to quit)");
            }
        }
    }
    Ok(())
}

/// Interactive chat loop: optional first message, then read lines from stdin until EOF or /exit.
/// Supports commands: /help, /exit, /quit. Ctrl+C also exits (SIGINT).
pub async fn run_chat_loop(
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
        run_one_turn(&runner, DEFAULT_THREAD_ID, &m, stream, false).await?;
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

        run_one_turn(&runner, DEFAULT_THREAD_ID, line, stream, true).await?;
        println!();
    }
    Ok(())
}
