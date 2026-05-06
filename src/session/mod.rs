//! Session management - create, save, restore, and list sessions

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod history;
pub mod manager;
pub mod search;

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub title: String,
    pub messages: Vec<crate::ai::Message>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Session {
    pub fn new() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
            title: "New Session".to_string(),
            messages: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn add_message(&mut self, msg: crate::ai::Message) {
        self.messages.push(msg);
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of a session (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
}
