//! ReAct agent: stubbed (empty implementations).

use anyhow::Result;
use std::path::Path;

/// User profile for long-term identity: injected as a System message before each turn when provided.
#[derive(Debug, Clone)]
pub struct UserProfile {
    pub user_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

impl UserProfile {
    /// Formats as the System message string used in checkpoint.
    pub fn to_system_content(&self) -> String {
        let name = [
            self.first_name.as_deref().unwrap_or("").trim(),
            self.last_name.as_deref().unwrap_or("").trim(),
        ]
        .join(" ")
        .trim()
        .to_string();
        let name_display = if name.is_empty() {
            "-".to_string()
        } else {
            name
        };
        let username_display = self
            .username
            .as_deref()
            .map(|u| format!("@{}", u.trim()))
            .unwrap_or_else(|| "-".to_string());
        format!(
            "User profile: {} ({}), user_id: {}",
            name_display, username_display, self.user_id
        )
    }
}

/// Runner stub; holds no state.
pub struct ReactRunner {}

/// Creates a ReactRunner stub. Does not touch the DB.
pub async fn create_react_runner(_db_path: impl AsRef<Path>) -> Result<ReactRunner> {
    Ok(ReactRunner {})
}

/// Prints nothing (stub).
pub async fn print_runtime_info(_db_path: impl AsRef<Path>) -> Result<()> {
    Ok(())
}

impl ReactRunner {
    /// Returns empty string (stub).
    pub async fn run_chat(
        &self,
        _thread_id: &str,
        _content: &str,
        _user_profile: Option<&UserProfile>,
    ) -> Result<String> {
        Ok(String::new())
    }

    /// Calls `on_chunk` with empty string and returns empty string (stub).
    pub async fn run_chat_stream(
        &self,
        _thread_id: &str,
        _content: &str,
        mut on_chunk: impl FnMut(&str) + Send,
        _user_profile: Option<&UserProfile>,
    ) -> Result<String> {
        on_chunk("");
        Ok(String::new())
    }
}
