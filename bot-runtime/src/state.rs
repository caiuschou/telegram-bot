use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Start,
    AwaitingInput,
    Custom(String),
}

#[derive(Clone)]
pub struct StateManager {
    states: Arc<RwLock<HashMap<i64, State>>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set(&self, user_id: i64, state: State) {
        self.states.write().await.insert(user_id, state);
    }

    pub async fn get(&self, user_id: i64) -> Option<State> {
        self.states.read().await.get(&user_id).cloned()
    }

    pub async fn remove(&self, user_id: i64) {
        self.states.write().await.remove(&user_id);
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}
