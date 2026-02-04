//! User info prefix format for embedding identity into User message content.
//!
//! Used when converting `SeedMessage` or `MessageRecord` into langgraph `Message::User(content)`:
//! content is prefixed with `[User: {display_name} / {username_part}] ` so the model can tell who
//! sent the message. Missing fields are rendered as `-`.
//!
//! **Interaction**: Consumed by `load::seed_messages_to_messages_with_user_info` (phase 1) and
//! by MessageRecord→Message conversion (phase 2). See [langgraph-memory-load-plan](../../docs/memory/plan/langgraph-memory-load-plan.md).

/// Format: `[User: {display_name} / {username_part}] `.
/// - `display_name`: `first_name` + optional `last_name` (space-separated, trimmed); if both missing, `-`.
/// - `username_part`: `@username` if present, otherwise `-`.
/// - A single space follows the closing `]` so the actual message content is separated.
///
/// **Example**: `[User: Alice Smith / @alice] Hello` or `[User: - / -] Hi`.
pub const USER_INFO_PREFIX_FORMAT: &str =
    "[User: {display_name} / {username_part}] ";

/// Builds the user info prefix string for a single User message.
///
/// - `first_name`: optional first name.
/// - `last_name`: optional last name (combined with first_name for display).
/// - `username`: optional username; rendered as `@value` or `-` if missing.
///
/// **Returns**: Prefix including a trailing space, e.g. `[User: Alice / @alice] `.
pub fn user_info_prefix(
    first_name: Option<&str>,
    last_name: Option<&str>,
    username: Option<&str>,
) -> String {
    let display_name = {
        let s = [
            first_name.unwrap_or("").trim(),
            last_name.unwrap_or("").trim(),
        ]
        .join(" ")
        .trim()
        .to_string();
        if s.is_empty() {
            "-".to_string()
        } else {
            s
        }
    };
    let username_part = username
        .and_then(|u| {
            let t = u.trim();
            if t.is_empty() {
                None
            } else {
                Some(format!("@{}", t))
            }
        })
        .unwrap_or_else(|| "-".to_string());
    format!("[User: {} / {}] ", display_name, username_part)
}

#[cfg(test)]
mod tests {
    use super::user_info_prefix;

    /// **Test: All fields present yields display name and @username.**
    #[test]
    fn user_info_prefix_all_fields() {
        let s = user_info_prefix(
            Some("Alice"),
            Some("Smith"),
            Some("alice"),
        );
        assert_eq!(s, "[User: Alice Smith / @alice] ");
    }

    /// **Test: First name only; last_name and username missing become -.**
    #[test]
    fn user_info_prefix_first_name_only() {
        let s = user_info_prefix(Some("Bob"), None, None);
        assert_eq!(s, "[User: Bob / -] ");
    }

    /// **Test: Username only; display name missing becomes -.**
    #[test]
    fn user_info_prefix_username_only() {
        let s = user_info_prefix(None, None, Some("bob"));
        assert_eq!(s, "[User: - / @bob] ");
    }

    /// **Test: All missing yields - / -.**
    #[test]
    fn user_info_prefix_all_missing() {
        let s = user_info_prefix(None, None, None);
        assert_eq!(s, "[User: - / -] ");
    }

    /// **Test: Empty strings are treated as missing (display - and -).**
    #[test]
    fn user_info_prefix_empty_strings() {
        let s = user_info_prefix(Some(""), Some(""), Some(""));
        assert_eq!(s, "[User: - / -] ");
    }

    /// **Test: Trailing space after bracket for content separation.**
    #[test]
    fn user_info_prefix_ends_with_space() {
        let s = user_info_prefix(Some("A"), None, Some("u"));
        assert!(s.ends_with(' '));
        assert_eq!(s, "[User: A / @u] ");
    }
}
