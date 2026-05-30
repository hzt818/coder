use rusqlite::{params, Connection, Result};
use tokio::task;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;

/// A very small SQLite-backed context store using an in-memory DB for tests.
pub struct SqliteContext {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteContext {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, message TEXT NOT NULL)",
            [],
        )?;
        Ok(SqliteContext { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn push(&self, msg: String) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            conn.execute("INSERT INTO messages (message) VALUES (?1)", params![msg])?;
            Ok(())
        })
        .await
        .unwrap()
    }

    pub async fn recent(&self, n: usize) -> Result<Vec<String>> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT message FROM messages ORDER BY id DESC LIMIT ?1")?;
            let rows = stmt
                .query_map(params![n as i64], |row| Ok(row.get::<_, String>(0)?))?
                .collect::<Result<Vec<String>, rusqlite::Error>>()?;
            // rows are newest-first; reverse to oldest-first
            let mut v = rows;
            v.reverse();
            Ok(v)
        })
        .await
        .unwrap()
    }
}

#[async_trait]
impl crate::context::ContextStore for SqliteContext {
    async fn push(&self, msg: String) {
        let _ = self.push(msg).await;
    }

    async fn recent(&self, n: usize) -> Vec<String> {
        match self.recent(n).await {
            Ok(v) => v,
            Err(_) => vec![],
        }
    }
}
