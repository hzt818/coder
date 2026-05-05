//! SQLite implementation of the Database trait using libsql

use async_trait::async_trait;

use super::db::{Database, Event, Memory};
use crate::session::{Session, SessionSummary};

/// SQLite-backed database using the libsql crate.
pub struct SqliteDb {
    conn: libsql::Connection,
}

impl SqliteDb {
    /// Open (or create) a SQLite database at the given path and run migrations.
    pub async fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid database path: {}", path.display()))?;

        let db = libsql::Database::open(path_str)?;
        let conn = db.connect()?;
        let instance = Self { conn };
        instance.run_migrations().await?;
        Ok(instance)
    }

    /// Run schema migrations.
    async fn run_migrations(&self) -> anyhow::Result<()> {
        super::migrate::run_migrations(&self.conn).await
    }
}

#[async_trait]
impl Database for SqliteDb {
    async fn save_session(&self, session: &Session) -> anyhow::Result<()> {
        let messages = serde_json::to_string(&session.messages)?;
        let metadata = serde_json::to_string(&session.metadata)?;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO sessions (id, title, created_at, updated_at, messages, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                libsql::params![
                    session.id.clone(),
                    session.title.clone(),
                    session.created_at.clone(),
                    session.updated_at.clone(),
                    messages,
                    metadata,
                ],
            )
            .await?;
        Ok(())
    }

    async fn load_session(&self, id: &str) -> anyhow::Result<Option<Session>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, title, created_at, updated_at, messages, metadata FROM sessions WHERE id = ?1",
                libsql::params![id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => {
                let session = Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    messages: {
                        let raw: String = row.get(4)?;
                        serde_json::from_str(&raw)?
                    },
                    metadata: {
                        let raw: String = row.get(5)?;
                        serde_json::from_str(&raw)?
                    },
                };
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn list_sessions(&self) -> anyhow::Result<Vec<SessionSummary>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, title, created_at, updated_at,
                        json_array_length(messages) as msg_count
                 FROM sessions ORDER BY updated_at DESC",
                libsql::params![],
            )
            .await?;

        let mut sessions = Vec::new();
        while let Some(row) = rows.next().await? {
            sessions.push(SessionSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                message_count: row.get::<i64>(4)? as usize,
            });
        }
        Ok(sessions)
    }

    async fn delete_session(&self, id: &str) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", libsql::params![id])
            .await?;
        Ok(())
    }

    async fn save_memory(&self, memory: &Memory) -> anyhow::Result<()> {
        let tags = serde_json::to_string(&memory.tags)?;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO memories (id, session_id, content, insight_type, tags, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                libsql::params![
                    memory.id.clone(),
                    memory.session_id.clone(),
                    memory.content.clone(),
                    memory.insight_type.clone(),
                    tags,
                    memory.created_at.clone(),
                    memory.updated_at.clone(),
                ],
            )
            .await?;
        Ok(())
    }

    async fn search_memory(&self, query: &str) -> anyhow::Result<Vec<Memory>> {
        let pattern = format!("%{}%", query);
        let mut rows = self
            .conn
            .query(
                "SELECT id, session_id, content, insight_type, tags, created_at, updated_at
                 FROM memories
                 WHERE content LIKE ?1 OR tags LIKE ?1
                 ORDER BY updated_at DESC
                 LIMIT 50",
                libsql::params![pattern],
            )
            .await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let tags_raw: String = row.get(4)?;
            results.push(Memory {
                id: row.get(0)?,
                session_id: row.get(1)?,
                content: row.get(2)?,
                insight_type: row.get(3)?,
                tags: serde_json::from_str(&tags_raw)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            });
        }
        Ok(results)
    }

    async fn list_session_memories(&self, session_id: &str) -> anyhow::Result<Vec<Memory>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, session_id, content, insight_type, tags, created_at, updated_at
                 FROM memories WHERE session_id = ?1
                 ORDER BY created_at ASC",
                libsql::params![session_id],
            )
            .await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let tags_raw: String = row.get(4)?;
            results.push(Memory {
                id: row.get(0)?,
                session_id: row.get(1)?,
                content: row.get(2)?,
                insight_type: row.get(3)?,
                tags: serde_json::from_str(&tags_raw)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            });
        }
        Ok(results)
    }

    async fn get_config(&self, key: &str) -> anyhow::Result<Option<String>> {
        let mut rows = self
            .conn
            .query(
                "SELECT value FROM config WHERE key = ?1",
                libsql::params![key],
            )
            .await?;

        match rows.next().await? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    async fn set_config(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
                libsql::params![key, value],
            )
            .await?;
        Ok(())
    }

    async fn track_event(&self, event: &Event) -> anyhow::Result<()> {
        let data = serde_json::to_string(&event.data)?;

        self.conn
            .execute(
                "INSERT INTO events (id, event_type, data, timestamp, session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                libsql::params![
                    event.id.clone(),
                    event.event_type.clone(),
                    data,
                    event.timestamp.clone(),
                    event.session_id.clone(),
                ],
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    async fn create_test_db() -> SqliteDb {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        SqliteDb::open(&path).await.unwrap()
    }

    #[tokio::test]
    async fn test_save_and_load_session() {
        let db = create_test_db().await;
        let mut session = Session::new();
        session.title = "Test Session".into();
        session.add_message(crate::ai::Message::user("Hello"));

        db.save_session(&session).await.unwrap();
        let loaded = db.load_session(&session.id).await.unwrap().unwrap();
        assert_eq!(loaded.title, "Test Session");
        assert_eq!(loaded.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let db = create_test_db().await;
        let session = Session::new();
        db.save_session(&session).await.unwrap();

        let list = db.list_sessions().await.unwrap();
        assert!(!list.is_empty());
        assert_eq!(list[0].id, session.id);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let db = create_test_db().await;
        let session = Session::new();
        db.save_session(&session).await.unwrap();
        db.delete_session(&session.id).await.unwrap();
        assert!(db.load_session(&session.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_crud() {
        let db = create_test_db().await;
        let memory = Memory {
            id: "mem-1".into(),
            session_id: "sess-1".into(),
            content: "Important insight about Rust async".into(),
            insight_type: "insight".into(),
            tags: vec!["rust".into(), "async".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };

        db.save_memory(&memory).await.unwrap();
        let results = db.search_memory("Rust").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "mem-1");

        let session_mems = db.list_session_memories("sess-1").await.unwrap();
        assert_eq!(session_mems.len(), 1);
    }

    #[tokio::test]
    async fn test_config() {
        let db = create_test_db().await;
        db.set_config("theme", "dark").await.unwrap();
        let val = db.get_config("theme").await.unwrap();
        assert_eq!(val, Some("dark".into()));

        let missing = db.get_config("nonexistent").await.unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_track_event() {
        let db = create_test_db().await;
        let event = Event {
            id: "evt-1".into(),
            event_type: "test".into(),
            data: serde_json::json!({"key": "value"}),
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: None,
        };
        db.track_event(&event).await.unwrap();
    }
}
