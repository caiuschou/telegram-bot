//! Generate MessageRecord-shaped seed messages (samples or synthetic).

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One message record; fields align with telegram-bot MessageRecord for import/export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedMessage {
    pub id: String,
    pub user_id: i64,
    pub chat_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub message_type: String,
    pub content: String,
    pub direction: String,
    pub created_at: DateTime<Utc>,
}

/// Embedded 100-message samples (conversation-samples-100).
const SAMPLES_JSON: &str = include_str!("samples.json");

/// Generates messages: by default uses built-in 100 samples; config from env.
/// - SEED_USE_SAMPLES: "1" (default) = use samples, "0" = generate synthetic
/// - SEED_MESSAGES_COUNT: limit count (default 100 when using samples)
/// - When synthetic: SEED_CHAT_ID, SEED_USER_ID_RECEIVED, SEED_USER_ID_SENT
pub fn generate_messages() -> Result<Vec<SeedMessage>> {
    let use_samples = std::env::var("SEED_USE_SAMPLES")
        .unwrap_or_else(|_| "1".into())
        .trim()
        == "1";
    let count = std::env::var("SEED_MESSAGES_COUNT")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(100);

    if use_samples {
        let all: Vec<SeedMessage> = serde_json::from_str(SAMPLES_JSON)?;
        let n = count.min(all.len());
        Ok(all.into_iter().take(n).collect())
    } else {
        generate_synthetic(count)
    }
}

fn generate_synthetic(n: usize) -> Result<Vec<SeedMessage>> {
    let chat_id: i64 = std::env::var("SEED_CHAT_ID")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(123456789);
    let user_received: i64 = std::env::var("SEED_USER_ID_RECEIVED")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(123456789);
    let user_sent: i64 = std::env::var("SEED_USER_ID_SENT")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(987654321);

    let mut out = Vec::with_capacity(n);
    let base_time = Utc::now() - chrono::Duration::seconds(15 * n as i64);
    for i in 0..n {
        let (user_id, username, first_name, direction) = if i % 2 == 0 {
            (user_received, Some("alice".into()), Some("Alice".into()), "received")
        } else {
            (user_sent, Some("bob".into()), Some("Bob".into()), "sent")
        };
        let created_at = base_time + chrono::Duration::seconds(15 * i as i64);
        out.push(SeedMessage {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            chat_id,
            username: username.clone(),
            first_name: first_name.clone(),
            last_name: None,
            message_type: "text".into(),
            content: format!("Seed message {}", i + 1),
            direction: direction.into(),
            created_at,
        });
    }
    Ok(out)
}
