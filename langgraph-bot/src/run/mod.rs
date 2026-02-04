//! Run Telegram bot. Uses telegram-bot with NoOpHandler (same pattern as telegram-simple-bot).

use anyhow::Result;
use std::sync::Arc;
use telegram_bot::{load_config, run_bot, NoOpHandler};

/// Runs the Telegram bot with a no-op handler (connects and runs REPL; does not reply to messages).
/// Config from env; `token` overrides BOT_TOKEN if provided. `_db` is unused (reserved for future use).
pub async fn run_telegram(_db: &std::path::Path, token: Option<String>) -> Result<()> {
    let config = load_config(token)?;
    run_bot(config, |_config, _components| Arc::new(NoOpHandler::new())).await
}
