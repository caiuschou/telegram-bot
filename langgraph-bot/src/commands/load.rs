//! Load subcommand: resolve message source (JSON path or Telegram DB), print preview. Does not import into checkpointer.
//!
//! When -m is not set: if TELEGRAM_MESSAGES_DB is set, load from that SQLite; else LANGGRAPH_MESSAGES_PATH (JSON).

use anyhow::{Context, Result};
use langgraph_bot::{
    load_all_messages_from_telegram_db, load_messages_from_path_with_user_info_with_stats,
    load_messages_from_telegram_db,
};
use std::path::PathBuf;

use super::memory::{print_messages_preview, MEMORY_PREVIEW_LEN};

const ENV_MESSAGES_PATH: &str = "LANGGRAPH_MESSAGES_PATH";
const ENV_TELEGRAM_MESSAGES_DB: &str = "TELEGRAM_MESSAGES_DB";

/// When -m is not set: if TELEGRAM_MESSAGES_DB is set, load from that SQLite; else LANGGRAPH_MESSAGES_PATH (JSON).
/// With Telegram DB: -t given → load that chat_id only, resolved_thread_id = that id; -t omitted → load all messages, resolved_thread_id = Some("all").
/// Returns (messages, skipped, resolved_thread_id). Caller uses resolved_thread_id as thread_id when present.
pub fn resolve_messages_source(
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
                format!(
                    "When using -t with Telegram DB, thread_id must be numeric (chat_id), got {:?}",
                    t
                )
            })?;
            let (messages, sk) = load_messages_from_telegram_db(db_path, chat_id, None)
                .with_context(|| format!("Load from Telegram DB {}", db_path))?;
            (messages, sk)
        } else {
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
        .with_context(|| {
            format!(
                "Set -m/--messages, or {}, or {} in .env",
                ENV_TELEGRAM_MESSAGES_DB, ENV_MESSAGES_PATH
            )
        })?;
    let (m, skipped) = load_messages_from_path_with_user_info_with_stats(&path)
        .with_context(|| format!("Load messages from {}", path.display()))?;
    Ok((m, skipped, None))
}

/// Load subcommand: resolve message source, print preview. Does not import into checkpointer.
pub fn cmd_load(
    messages_path: Option<PathBuf>,
    _db: &std::path::Path,
    thread_id: Option<String>,
) -> Result<()> {
    let (messages, skipped, resolved_thread_id) =
        resolve_messages_source(messages_path, thread_id.as_deref())?;
    if skipped > 0 {
        eprintln!(
            "Warning: {} messages skipped (direction not received/sent)",
            skipped
        );
    }
    let _thread_id = resolved_thread_id
        .or(thread_id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    println!("Loaded {} messages (not imported into checkpoint)", messages.len());
    print_messages_preview(&messages, MEMORY_PREVIEW_LEN.min(10));
    Ok(())
}
