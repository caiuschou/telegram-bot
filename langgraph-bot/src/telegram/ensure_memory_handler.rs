//! Handler that ensures bot identity and user profile are in long-term memory (if not already).
//!
//! **Logic (if not, then write):** Bot info is written under the bot's Telegram user_id (once per
//! process). The current message user's profile is written under that user's user_id (once per
//! user). Uses [`ReactRunner::ensure_remember`] to run a minimal turn so the model can remember.
//!
//! Runs before the agent: in the Telegram handler chain, this handler runs first and returns
//! `Continue`, then the inner handler (e.g. AgentHandler) runs.
//!
//! **Potential issues:**
//! - **Latency:** First message (and first message per user) runs one full ReAct turn for ensure
//!   before the agent runs, so the user waits longer.
//! - **Model compliance:** We rely on the model calling the remember tool in that one turn; if it
//!   does not, the content may not be stored. No verification.
//! - **Cost:** Ensure runs a full stream (LLM + possible tool call) per ensure; relatively heavy
//!   for storing a short string.

use crate::react::UserProfile;
use crate::ReactRunner;
use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use telegram_bot::{Handler, HandlerResponse, Message, Result, User as TelegramUser};
use tracing::{debug, info};

/// Ensures bot identity and user profile are written to long-term memory when not already present.
/// Does not reply to the user; always returns `Continue`. Compose with [`super::AgentHandler`] so
/// ensure runs first, then the agent.
pub struct EnsureLongTermMemoryHandler {
    runner: Arc<ReactRunner>,
    bot_user: Arc<tokio::sync::RwLock<Option<TelegramUser>>>,
    /// True after we have run ensure for bot at least once (per process).
    bot_ensured: AtomicBool,
    /// User ids for which we have already run ensure (once per user).
    ensured_user_ids: dashmap::DashSet<String>,
}

impl EnsureLongTermMemoryHandler {
    pub fn new(
        runner: Arc<ReactRunner>,
        bot_user: Arc<tokio::sync::RwLock<Option<TelegramUser>>>,
    ) -> Self {
        Self {
            runner,
            bot_user,
            bot_ensured: AtomicBool::new(false),
            ensured_user_ids: dashmap::DashSet::new(),
        }
    }

    fn format_bot_identity(u: &TelegramUser) -> String {
        let name = [
            u.first_name.as_deref().unwrap_or("").trim(),
            u.last_name.as_deref().unwrap_or("").trim(),
        ]
        .join(" ")
        .trim()
        .to_string();
        let name_display = if name.is_empty() {
            "-".to_string()
        } else {
            name
        };
        let username_display = u
            .username
            .as_deref()
            .map(|s| format!("@{}", s.trim()))
            .unwrap_or_else(|| "-".to_string());
        format!(
            "Bot identity: {} ({}), user_id: {}",
            name_display,
            username_display,
            u.id
        )
    }

    async fn ensure_bot(&self) -> AnyhowResult<()> {
        if self.bot_ensured.swap(true, Ordering::Relaxed) {
            return Ok(());
        }
        let bot = self.bot_user.read().await.clone();
        let Some(ref u) = bot else {
            return Ok(());
        };
        let content = Self::format_bot_identity(u);
        let user_id = u.id.to_string();
        info!(bot_id = %user_id, "Ensure long-term memory: bot identity");
        if let Err(e) = self.runner.ensure_remember(&user_id, &content).await {
            self.bot_ensured.store(false, Ordering::Relaxed);
            return Err(e);
        }
        Ok(())
    }

}

#[async_trait]
impl Handler for EnsureLongTermMemoryHandler {
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        if let Err(e) = self.ensure_bot().await {
            tracing::error!(error = %e, "Ensure bot identity in long-term memory failed");
        }
        let profile = UserProfile {
            user_id: message.user.id.to_string(),
            first_name: message.user.first_name.clone(),
            last_name: message.user.last_name.clone(),
            username: message.user.username.clone(),
        };
        // Only one task per user_id runs ensure; insert() is true only when we are first.
        if self.ensured_user_ids.insert(profile.user_id.clone()) {
            let content = profile.to_system_content();
            if let Err(e) = self.runner.ensure_remember(&profile.user_id, &content).await {
                self.ensured_user_ids.remove(&profile.user_id);
                tracing::error!(error = %e, user_id = %profile.user_id, "Ensure user profile in long-term memory failed");
            } else {
                debug!(user_id = %profile.user_id, "Ensure long-term memory: user profile");
            }
        }
        Ok(HandlerResponse::Continue)
    }
}

// ---------- Composite: Ensure then Agent ----------

/// Runs [`EnsureLongTermMemoryHandler`] then [`super::AgentHandler`]. Inject this as the single handler so ensure runs before the agent.
pub struct EnsureThenAgentHandler {
    ensure: EnsureLongTermMemoryHandler,
    agent: super::AgentHandler,
}

impl EnsureThenAgentHandler {
    pub fn new(
        ensure: EnsureLongTermMemoryHandler,
        agent: super::AgentHandler,
    ) -> Self {
        Self { ensure, agent }
    }
}

#[async_trait]
impl Handler for EnsureThenAgentHandler {
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        self.ensure.handle(message).await?;
        self.agent.handle(message).await
    }
}
