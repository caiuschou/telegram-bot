//! Load messages from Telegram bot SQLite (messages table) into langgraph `Message` list.
//!
//! Schema must match telegram-bot MessageRepository: id, user_id, chat_id, username, first_name,
//! last_name, message_type, content, direction, created_at. Used when load is run without
//! `-m` and `TELEGRAM_MESSAGES_DB` is set (default source = long-term store).

use anyhow::Result;
use langgraph::Message;
use rusqlite::Connection;
use std::path::Path;

use crate::format::user_info_prefix;

/// Default limit when loading from Telegram DB (chronological order, oldest first).
const DEFAULT_LIMIT: u32 = 1000;

/// Loads all messages from the Telegram bot `messages` table (no chat_id filter), in global chronological order (created_at ASC).
///
/// Used when load is run without `-t`: import every message from the DB into one thread. Same conversion as `load_messages_from_telegram_db` (received→User with prefix, sent→Assistant).
pub fn load_all_messages_from_telegram_db(
    db_path: impl AsRef<Path>,
    limit: Option<u32>,
) -> Result<(Vec<Message>, usize)> {
    let conn = Connection::open(db_path.as_ref())
        .map_err(|e| anyhow::anyhow!("open telegram messages db: {}", e))?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as i64;

    let mut stmt = conn.prepare(
        r#"
        SELECT user_id, chat_id, username, first_name, last_name, content, direction
        FROM messages
        ORDER BY created_at ASC
        LIMIT ?
        "#,
    )?;
    let rows = stmt.query_map([limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;

    let mut messages = Vec::new();
    let mut skipped = 0usize;
    for row in rows {
        let (_, _, username, first_name, last_name, content, direction) = row?;
        let msg = match direction.as_str() {
            "received" => {
                let prefix = user_info_prefix(
                    first_name.as_deref(),
                    last_name.as_deref(),
                    username.as_deref(),
                );
                Message::User(format!("{}{}", prefix, content))
            }
            "sent" => Message::Assistant(content),
            _ => {
                skipped += 1;
                continue;
            }
        };
        messages.push(msg);
    }
    Ok((messages, skipped))
}

/// Loads messages from the Telegram bot `messages` table for the given `chat_id`, in chronological order.
///
/// - `direction == "received"` → `Message::User(prefix + content)` with user info prefix.
/// - `direction == "sent"` → `Message::Assistant(content)`.
/// Other directions are skipped. Returns (messages, skipped_count).
///
/// **Interaction**: Caller ensures `db_path` points to the same schema as telegram-bot's messages table.
pub fn load_messages_from_telegram_db(
    db_path: impl AsRef<Path>,
    chat_id: i64,
    limit: Option<u32>,
) -> Result<(Vec<Message>, usize)> {
    let conn = Connection::open(db_path.as_ref())
        .map_err(|e| anyhow::anyhow!("open telegram messages db: {}", e))?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as i64;

    let mut stmt = conn.prepare(
        r#"
        SELECT user_id, chat_id, username, first_name, last_name, content, direction
        FROM messages
        WHERE chat_id = ?
        ORDER BY created_at ASC
        LIMIT ?
        "#,
    )?;
    let rows = stmt.query_map([chat_id, limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;

    let mut messages = Vec::new();
    let mut skipped = 0usize;
    for row in rows {
        let (_, _, username, first_name, last_name, content, direction) = row?;
        let msg = match direction.as_str() {
            "received" => {
                let prefix = user_info_prefix(
                    first_name.as_deref(),
                    last_name.as_deref(),
                    username.as_deref(),
                );
                Message::User(format!("{}{}", prefix, content))
            }
            "sent" => Message::Assistant(content),
            _ => {
                skipped += 1;
                continue;
            }
        };
        messages.push(msg);
    }
    Ok((messages, skipped))
}

#[cfg(test)]
mod tests {
    use super::load_all_messages_from_telegram_db;

    /// **Test: load_all_messages_from_telegram_db returns empty list for empty table.**
    #[test]
    fn load_all_empty_returns_empty() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let db_path = dir.path().join("empty.db");
        let conn = rusqlite::Connection::open(&db_path).expect("open");
        conn.execute_batch(
            r#"
            CREATE TABLE messages (id TEXT, user_id INTEGER, chat_id INTEGER, username TEXT, first_name TEXT, last_name TEXT, message_type TEXT, content TEXT, direction TEXT, created_at TEXT);
            "#,
        )
        .expect("create table");
        let (messages, skipped) =
            load_all_messages_from_telegram_db(&db_path, None).expect("query");
        assert!(messages.is_empty());
        assert_eq!(skipped, 0);
    }

    /// **Test: load_all_messages_from_telegram_db returns all messages in created_at order.**
    #[test]
    fn load_all_returns_messages_in_order() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let db_path = dir.path().join("with_messages.db");
        let conn = rusqlite::Connection::open(&db_path).expect("open");
        conn.execute_batch(
            r#"
            CREATE TABLE messages (id TEXT, user_id INTEGER, chat_id INTEGER, username TEXT, first_name TEXT, last_name TEXT, message_type TEXT, content TEXT, direction TEXT, created_at TEXT);
            INSERT INTO messages (id, user_id, chat_id, username, first_name, last_name, message_type, content, direction, created_at)
            VALUES ('1', 1, 100, NULL, NULL, NULL, 'text', 'a', 'received', '2025-01-01T00:00:00Z');
            INSERT INTO messages (id, user_id, chat_id, username, first_name, last_name, message_type, content, direction, created_at)
            VALUES ('2', 1, 200, NULL, NULL, NULL, 'text', 'b', 'sent', '2025-01-02T00:00:00Z');
            "#,
        )
        .expect("create and insert");
        let (messages, _) = load_all_messages_from_telegram_db(&db_path, None).expect("query");
        assert_eq!(messages.len(), 2);
        // First message: received -> User with prefix
        assert!(matches!(&messages[0], langgraph::Message::User(_)));
        // Second: sent -> Assistant
        assert!(matches!(&messages[1], langgraph::Message::Assistant(_)));
    }
}
