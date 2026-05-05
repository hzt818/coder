//! Database trait and shared types for persistence operations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A persistent memory entry stored across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub session_id: String,
    pub content: String,
    pub insight_type: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A telemetry or analytics event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
    pub session_id: Option<String>,
}

/// Database trait defining all persistence operations.
///
/// Implementations handle storage details (SQLite, PostgreSQL, etc.)
/// while business logic depends only on this abstract interface.
#[async_trait]
pub trait Database: Send + Sync {
    /// Save or replace a session.
    async fn save_session(&self, session: &crate::session::Session) -> anyhow::Result<()>;

    /// Load a session by ID, returning None if not found.
    async fn load_session(&self, id: &str) -> anyhow::Result<Option<crate::session::Session>>;

    /// List all saved sessions, ordered by most recent first.
    async fn list_sessions(&self) -> anyhow::Result<Vec<crate::session::SessionSummary>>;

    /// Delete a session by ID.
    async fn delete_session(&self, id: &str) -> anyhow::Result<()>;

    /// Store a memory entry.
    async fn save_memory(&self, memory: &Memory) -> anyhow::Result<()>;

    /// Search memory entries by keyword query.
    async fn search_memory(&self, query: &str) -> anyhow::Result<Vec<Memory>>;

    /// List all memory entries for a session.
    async fn list_session_memories(&self, session_id: &str) -> anyhow::Result<Vec<Memory>>;

    /// Get a configuration value.
    async fn get_config(&self, key: &str) -> anyhow::Result<Option<String>>;

    /// Set a configuration value.
    async fn set_config(&self, key: &str, value: &str) -> anyhow::Result<()>;

    /// Record a telemetry event.
    async fn track_event(&self, event: &Event) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_serialization() {
        let memory = Memory {
            id: "mem-1".into(),
            session_id: "sess-1".into(),
            content: "Important insight".into(),
            insight_type: "insight".into(),
            tags: vec!["rust".into(), "async".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&memory).unwrap();
        let deserialized: Memory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "mem-1");
        assert_eq!(deserialized.tags.len(), 2);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event {
            id: "evt-1".into(),
            event_type: "tool_exec".into(),
            data: serde_json::json!({"tool": "bash", "duration_ms": 150}),
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: Some("sess-1".into()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, "tool_exec");
        assert!(deserialized.session_id.is_some());
    }
}
