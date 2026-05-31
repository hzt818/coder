//! SQLite-backed context storage
//!
//! Persists messages to a SQLite database for durable context tracking.

use async_trait::async_trait;
use rusqlite::{params, Connection, Result as SqlResult};
use std::sync::{Arc, Mutex};
use tokio::task;

use super::store::ContextStore;

/// SQLite-backed message context store.
///
/// Uses `rusqlite` for synchronous SQLite access, dispatched to
/// `spawn_blocking` so it does not block the Tokio runtime.
pub struct SqliteContext {
    conn: Arc<Mutex<Connection>>,
    table_name: String,
}

impl SqliteContext {
    /// Create an in-memory SQLite context store.
    ///
    /// Useful for tests and short-lived sessions that don't need
    /// durable persistence.
    pub fn new_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            table_name: "messages".to_string(),
        })
    }

    async fn push_inner(&self, msg: &str) -> SqlResult<()> {
        let conn = Arc::clone(&self.conn);
        let table = self.table_name.clone();
        let msg_owned = msg.to_owned();
        task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            conn.execute(
                &format!("INSERT INTO {} (message) VALUES (?1)", table),
                params![msg_owned],
            )?;
            Ok(())
        })
        .await
        .unwrap()
    }

    async fn recent_inner(&self, n: usize) -> SqlResult<Vec<String>> {
        let conn = Arc::clone(&self.conn);
        let table = self.table_name.clone();
        task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(&format!(
                "SELECT message FROM {} ORDER BY id DESC LIMIT ?1",
                table
            ))?;
            let mut rows = stmt
                .query_map(params![n as i64], |row| row.get::<_, String>(0))?
                .collect::<SqlResult<Vec<String>>>()?;
            rows.reverse();
            Ok(rows)
        })
        .await
        .unwrap()
    }
}

#[async_trait]
impl ContextStore for SqliteContext {
    async fn push(&self, msg: String) {
        let _ = self.push_inner(&msg).await;
    }

    async fn recent(&self, n: usize) -> Vec<String> {
        self.recent_inner(n).await.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_roundtrip() {
        let ctx = SqliteContext::new_in_memory().expect("failed to create in-memory SQLite context");
        ctx.push("x".to_string()).await;
        ctx.push("y".to_string()).await;
        let r = ctx.recent(10).await;
        assert_eq!(r, vec!["x".to_string(), "y".to_string()]);
    }

    #[tokio::test]
    async fn sqlite_recent_ordering() {
        let ctx = SqliteContext::new_in_memory().expect("failed to create in-memory SQLite context");
        for i in 0..5 {
            ctx.push(format!("msg-{}", i)).await;
        }
        let r = ctx.recent(3).await;
        assert_eq!(r, vec!["msg-2".to_string(), "msg-3".to_string(), "msg-4".to_string()]);
    }
}
