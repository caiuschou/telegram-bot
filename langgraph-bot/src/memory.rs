//! Long-term memory policy: controls whether the ReAct runner gets a vector store and
//! remember/recall tools via upstream `build_react_run_context`.
//!
//! When [`LongTermMemoryPolicy::disabled()`] is used, `user_id` is `None` and the runner
//! has no long-term memory. When [`LongTermMemoryPolicy::from_env()`] is used, `user_id`
//! is read from `USER_ID` (default `"1"`) so that upstream can create store and tools
//! when embedding is also configured.

/// Policy for enabling or disabling long-term memory (vector store + remember/recall tools).
///
/// The runner passes this as `user_id` into `ReactBuildConfig`. Upstream
/// `build_react_run_context` only creates store and memory tools when `user_id` is `Some`
/// and embedding config is available.
#[derive(Debug, Clone)]
pub struct LongTermMemoryPolicy {
    /// When `Some`, long-term memory can be enabled by upstream if embedding is configured.
    /// When `None`, long-term memory is disabled.
    pub(super) user_id: Option<String>,
}

impl LongTermMemoryPolicy {
    /// Long-term memory disabled: no store, no remember/recall tools.
    pub fn disabled() -> Self {
        Self { user_id: None }
    }

    /// Long-term memory policy from environment: `USER_ID` if set, otherwise `"1"`.
    /// Caller (e.g. `react_build_config_for_runner`) combines this with embedding
    /// availability to decide whether upstream returns store and tools.
    pub fn from_env() -> Self {
        let user_id = std::env::var("USER_ID").ok().or_else(|| Some("1".to_string()));
        Self { user_id }
    }

    /// Returns true when `user_id` is set (long-term memory may be enabled by upstream
    /// if embedding is also configured).
    pub fn is_enabled(&self) -> bool {
        self.user_id.is_some()
    }

    /// Returns the user id for the long-term store, if any.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_has_no_user_id_and_is_not_enabled() {
        let policy = LongTermMemoryPolicy::disabled();
        assert!(!policy.is_enabled());
        assert_eq!(policy.user_id(), None);
    }

    #[test]
    fn from_env_returns_policy_with_user_id_from_env_or_default() {
        let policy = LongTermMemoryPolicy::from_env();
        // With no USER_ID set, from_env uses default "1"
        let uid = policy.user_id();
        assert!(uid.is_some());
        assert!(policy.is_enabled());
    }
}
